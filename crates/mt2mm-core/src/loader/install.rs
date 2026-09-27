use super::identify::{identify_file, is_loader_core, missing_exports, DllKind};
use super::{GameFolder, CORE_FILE, LOADER_FOLDER, LOADER_FOLDER_FILES, PROXY_FILE, TEMPORARY_SUFFIX};
use crate::game_process::game_running;
use anyhow::{bail, Context, Result};
use std::collections::BTreeSet;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

pub struct LoaderFiles {
    pub proxy: PathBuf,
    pub core: PathBuf,
    pub version: String,
    exports: BTreeSet<String>,
}

const VERIFY_ADVICE: &str = "Use Steam's \"Verify integrity of game files\" to get it back.";
const NOTHING_CHANGED: &str = "Nothing was changed.";


fn temporary_path(target: &Path) -> PathBuf {
    let mut name = OsString::from(target.as_os_str());
    name.push(TEMPORARY_SUFFIX);

    PathBuf::from(name)
}

// Written next to the target and renamed over it, so an interrupted copy never leaves a half-written DLL.
fn copy_atomically(source: &Path, target: &Path) -> Result<()> {
    let bytes = std::fs::read(source).with_context(|| format!("reading {}", source.display()))?;
    let temporary = temporary_path(target);

    std::fs::write(&temporary, &bytes).with_context(|| format!("writing {}", temporary.display()))?;

    if let Err(error) = std::fs::rename(&temporary, target) {
        let _ = std::fs::remove_file(&temporary);

        return Err(error).with_context(|| format!("replacing {}", target.display()));
    }

    Ok(())
}

fn remove_if_present(path: &Path) -> Result<bool> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error).with_context(|| format!("removing {}", path.display())),
    }
}


fn ensure_game_folder(folder: &GameFolder) -> Result<()> {
    if !folder.exe().is_file() {
        bail!("{} has no MT2.exe, so it isn't the game folder. {NOTHING_CHANGED}", folder.root.display());
    }

    if game_running(Some(&folder.root)) {
        bail!("MMORPG Tycoon 2 is running. Close it first: Windows keeps the game's files locked while it runs. {NOTHING_CHANGED}");
    }

    Ok(())
}

fn ensure_exports_cover(zlib_exports: &BTreeSet<String>, files: &LoaderFiles) -> Result<()> {
    let missing = missing_exports(zlib_exports, &files.exports);

    if !missing.is_empty() {
        bail!(
            "The game's zlib1.dll has functions this loader ({}) doesn't have: {}. A newer loader is needed. {NOTHING_CHANGED}",
            files.version,
            missing.join(", ")
        );
    }

    Ok(())
}


pub fn find_loader_files(explicit: Option<&Path>) -> Result<LoaderFiles> {
    let folder = match explicit {
        Some(folder) => folder.to_path_buf(),
        None => std::env::current_exe()?.parent().context("the manager's own folder is unknown")?.join("loader"),
    };

    let proxy = folder.join(PROXY_FILE);
    let core = folder.join(LOADER_FOLDER).join(CORE_FILE);

    let DllKind::LoaderProxy { version, exports } = identify_file(&proxy) else {
        bail!("{} doesn't hold the loader (zlib1.dll and mt2loader\\mt2loader.dll).", folder.display());
    };

    if !is_loader_core(&core) {
        bail!("{} is missing or isn't the loader's core.", core.display());
    }

    Ok(LoaderFiles { proxy, core, version, exports })
}


pub fn install(install_dir: &Path, files: &LoaderFiles) -> Result<Vec<String>> {
    let folder = GameFolder::new(install_dir);
    ensure_game_folder(&folder)?;

    let mut actions = Vec::new();
    let mut save_backup = false;

    match identify_file(&folder.proxy()) {
        DllKind::GameZlib { exports } => {
            ensure_exports_cover(&exports, files)?;
            save_backup = true;
        }
        DllKind::LoaderProxy { version, .. } => actions.push(format!("Replacing loader {version}")),
        DllKind::Missing => bail!("The game folder has no zlib1.dll. {VERIFY_ADVICE} {NOTHING_CHANGED}"),
        DllKind::Unknown { reason } => {
            bail!("The game folder's zlib1.dll isn't the game's or this loader's ({reason}). Another tool may have replaced it. {NOTHING_CHANGED}")
        }
    }

    std::fs::create_dir_all(folder.loader_folder()).context("creating the mt2loader folder")?;

    // Only the manager's Remove needs this copy; the loader itself never does.
    if save_backup {
        copy_atomically(&folder.proxy(), &folder.backup())?;
        actions.push("Saved the game's zlib1.dll as mt2loader\\zlib1.game.dll".to_string());
    }

    copy_atomically(&files.core, &folder.core())?;
    actions.push(format!("Copied mt2loader\\mt2loader.dll {}", files.version));

    // Last, so the game only ever loads the proxy once everything it needs is in place.
    copy_atomically(&files.proxy, &folder.proxy())?;
    actions.push(format!("Installed the loader {} as zlib1.dll", files.version));

    Ok(actions)
}


fn remove_loader_folder(folder: &GameFolder, actions: &mut Vec<String>) -> Result<()> {
    let loader_folder = folder.loader_folder();

    let Ok(metadata) = std::fs::symlink_metadata(&loader_folder) else {
        return Ok(());
    };

    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        actions.push("Left mt2loader in place: it isn't a plain folder".to_string());

        return Ok(());
    }

    for name in LOADER_FOLDER_FILES {
        let path = loader_folder.join(name);
        remove_if_present(&path)?;
        remove_if_present(&temporary_path(&path))?;
    }

    let remaining: Vec<String> = std::fs::read_dir(&loader_folder)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();

    if remaining.is_empty() {
        std::fs::remove_dir(&loader_folder).context("removing the mt2loader folder")?;
        actions.push("Removed the mt2loader folder".to_string());

        return Ok(());
    }

    actions.push(format!("Left mt2loader in place: it holds files the loader didn't make ({})", remaining.join(", ")));

    Ok(())
}

pub fn remove(install_dir: &Path) -> Result<Vec<String>> {
    let folder = GameFolder::new(install_dir);
    ensure_game_folder(&folder)?;

    let mut actions = Vec::new();

    match identify_file(&folder.proxy()) {
        DllKind::LoaderProxy { .. } => {
            if !matches!(identify_file(&folder.backup()), DllKind::GameZlib { .. }) {
                bail!(
                    "The game's own zlib1.dll wasn't saved (the loader was installed by hand, or the copy is damaged). \
                     Use Steam's \"Verify integrity of game files\": it puts zlib1.dll back. Then remove the loader again to tidy up."
                );
            }

            copy_atomically(&folder.backup(), &folder.proxy())?;
            actions.push("Put the game's own zlib1.dll back".to_string());
        }
        DllKind::GameZlib { .. } => actions.push("zlib1.dll is already the game's own".to_string()),
        DllKind::Missing => bail!("The game folder has no zlib1.dll. {VERIFY_ADVICE}"),
        DllKind::Unknown { reason } => {
            bail!("The game folder's zlib1.dll isn't the game's or this loader's ({reason}). Another tool may have replaced it. {NOTHING_CHANGED}")
        }
    }

    if remove_if_present(&folder.legacy_original())? {
        actions.push("Deleted zlib1_original.dll".to_string());
    }

    remove_if_present(&temporary_path(&folder.proxy()))?;
    remove_loader_folder(&folder, &mut actions)?;

    Ok(actions)
}
