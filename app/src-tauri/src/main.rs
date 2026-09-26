// Hide the console window on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use mt2mm_core::build::BuildPlan;
use mt2mm_core::library::Imported;
use mt2mm_core::paths::{self, GamePaths};
use mt2mm_core::session::{Config, Session, Status};
use std::path::PathBuf;

type Res<T> = Result<T, String>;

#[derive(serde::Serialize)]
struct Launched {
    plan: BuildPlan,
    applied: bool,
}


fn err(e: anyhow::Error) -> String {
    format!("{e:#}")
}

fn session() -> Session {
    Session::open(&Config::default())
}


// Commands are async so that Tauri runs them off the UI thread.
#[tauri::command]
async fn status() -> Status {
    session().status()
}

#[tauri::command]
async fn set_enabled(id: String, enabled: bool) -> Res<()> {
    session().library().and_then(|l| l.set_enabled(&id, enabled)).map_err(err)
}

#[tauri::command]
async fn set_order(order: Vec<String>) -> Res<()> {
    session().library().and_then(|l| l.set_order(&order)).map_err(err)
}

#[tauri::command]
async fn apply_mods(ids: Vec<String>) -> Res<()> {
    session().library().and_then(|l| l.apply(&ids)).map_err(err)
}

#[tauri::command]
async fn unapply_mods(ids: Vec<String>) -> Res<()> {
    session().library().and_then(|l| l.unapply(&ids)).map_err(err)
}


#[tauri::command]
async fn add_mods(paths: Vec<String>) -> Res<Vec<Imported>> {
    let lib = session().library().map_err(err)?;
    let mut out = Vec::new();
    for p in paths.iter().map(PathBuf::from) {
        let single = !p.is_dir() || p.join(mt2mm_core::manifest::MANIFEST_FILE).is_file();
        let res = if single { lib.import(&p, false).map(|r| vec![r]) } else { lib.import_all(&p) };
        match res {
            Ok(v) if v.is_empty() => out.push(Imported {
                source: p,
                id: None,
                updated: false,
                error: Some("no mods found here (a mod is a folder or .zip with a manifest.json)".into()),
            }),
            Ok(v) => out.extend(v),
            Err(e) => out.push(Imported { source: p, id: None, updated: false, error: Some(err(e)) }),
        }
    }
    Ok(out)
}

#[tauri::command]
async fn adopt_mod(path: String) -> Res<Imported> {
    let s = session();
    let p = PathBuf::from(&path);
    let mod_dir = s.paths.mod_dir().ok_or("mod folder not found")?;
    let inside = p.canonicalize().ok().and_then(|c| c.parent().map(|x| x.to_path_buf())) == mod_dir.canonicalize().ok();
    if !inside {
        return Err(format!("{path} is not in the game's mod folder"));
    }
    s.library().and_then(|l| l.import(&p, true)).map_err(err)
}

#[tauri::command]
async fn add_dev_mod(path: String) -> Res<Imported> {
    session().library().and_then(|l| l.import_dev(&PathBuf::from(path))).map_err(err)
}

#[tauri::command]
async fn refresh_dev_mods(ids: Vec<String>) -> Res<Vec<Imported>> {
    let lib = session().library().map_err(err)?;
    let ids = if ids.is_empty() { lib.load_state().dev_sources.into_keys().collect() } else { ids };
    Ok(ids
        .into_iter()
        .map(|id| {
            lib.refresh_dev(&id).unwrap_or_else(|e| Imported { source: PathBuf::from(&id), id: Some(id), updated: false, error: Some(err(e)) })
        })
        .collect())
}

#[tauri::command]
async fn stop_dev_mod(id: String) -> Res<()> {
    session().library().and_then(|l| l.stop_dev(&id)).map_err(err)
}

#[tauri::command]
async fn remove_mod(id: String) -> Res<()> {
    session().library().and_then(|l| l.remove(&id)).map_err(err)
}

#[tauri::command]
async fn set_mod_settings(
    id: String,
    changes: std::collections::BTreeMap<String, serde_json::Value>,
    reset: Vec<String>,
) -> Res<()> {
    session().library().and_then(|l| l.set_settings(&id, &changes, &reset)).map_err(err)
}

#[tauri::command]
async fn mod_icon(id: String) -> Res<tauri::ipc::Response> {
    let mods = session().library().and_then(|l| l.list()).map_err(err)?;
    let m = mods.iter().find(|m| m.id() == Some(id.as_str())).ok_or("no such mod")?;
    let p = m.icon_path().ok_or("no icon")?;
    let bytes = std::fs::read(p).map_err(|e| e.to_string())?;
    Ok(tauri::ipc::Response::new(bytes))
}


#[tauri::command]
async fn create_profile(name: String, copy_active: bool) -> Res<String> {
    let lib = session().library().map_err(err)?;
    let active = lib.load_state().active;
    lib.create_profile(&name, copy_active.then_some(active.as_str())).map_err(err)
}

#[tauri::command]
async fn switch_profile(name: String) -> Res<()> {
    session().library().and_then(|l| l.switch_profile(&name)).map_err(err)
}

#[tauri::command]
async fn rename_profile(old: String, new: String) -> Res<()> {
    session().library().and_then(|l| l.rename_profile(&old, &new)).map_err(err)
}

#[tauri::command]
async fn delete_profile(name: String) -> Res<()> {
    session().library().and_then(|l| l.delete_profile(&name)).map_err(err)
}


#[tauri::command]
async fn check() -> Res<BuildPlan> {
    session().plan().map_err(err)
}

#[tauri::command]
async fn deploy() -> Res<BuildPlan> {
    session().deploy().map_err(err)
}

#[tauri::command]
async fn launch() -> Res<Launched> {
    let (plan, applied) = session().launch_checked().map_err(err)?;
    Ok(Launched { plan, applied })
}

#[tauri::command]
async fn clean() -> Res<usize> {
    session().clean().map(|v| v.len()).map_err(err)
}

#[tauri::command]
fn get_config() -> Config {
    Config::load()
}

#[tauri::command]
fn save_config(config: Config) -> Res<()> {
    config.save().map_err(err)
}

#[tauri::command]
async fn game_running() -> bool {
    mt2mm_core::game_process::game_running(session().paths.install_dir.as_deref())
}

#[tauri::command]
async fn detect_paths(install_dir: Option<String>) -> GamePaths {
    let detected_install = paths::find_install_dir();
    let data_zip = install_dir
        .map(PathBuf::from)
        .or_else(|| detected_install.clone())
        .and_then(|dir| paths::find_data_zip(&dir));

    GamePaths {
        install_dir: detected_install,
        data_zip,
        profile_dir: paths::find_profile_dir(),
    }
}

#[tauri::command]
async fn open_folder(which: String) -> Res<()> {
    let s = session();
    let p = match which.as_str() {
        "mod" => s.paths.mod_dir(),
        "library" => s.paths.manager_dir().map(|d| d.join("library")),
        "profile" => s.paths.profile_dir.clone(),
        "install" => s.paths.install_dir.clone(),
        w => match w.strip_prefix("mod:") {
            Some(id) => s.library().and_then(|l| l.list()).ok().and_then(|v| v.into_iter().find(|m| m.id() == Some(id)).map(|m| m.dir)),
            None => None,
        },
    }
    .ok_or("folder not found")?;
    std::fs::create_dir_all(&p).map_err(|e| e.to_string())?;
    mt2mm_core::util::open_external(p.as_os_str()).map_err(err)
}

#[tauri::command]
async fn open_url(url: String) -> Res<()> {
    let u = url.trim();
    let lower = u.to_ascii_lowercase();
    let ok = ["https://", "http://", "mailto:"].iter().any(|p| lower.starts_with(p)) && !u.chars().any(|c| c.is_whitespace() || c == '"');
    if !ok {
        return Err(format!("not a web link: {u}"));
    }
    mt2mm_core::util::open_external(std::ffi::OsStr::new(u)).map_err(err)
}


fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            status,
            set_enabled,
            set_order,
            apply_mods,
            unapply_mods,
            add_mods,
            adopt_mod,
            add_dev_mod,
            refresh_dev_mods,
            stop_dev_mod,
            remove_mod,
            set_mod_settings,
            mod_icon,
            create_profile,
            switch_profile,
            rename_profile,
            delete_profile,
            check,
            deploy,
            launch,
            clean,
            get_config,
            save_config,
            detect_paths,
            game_running,
            open_folder,
            open_url
        ])
        .run(tauri::generate_context!())
        .expect("error while running the mod manager");
}
