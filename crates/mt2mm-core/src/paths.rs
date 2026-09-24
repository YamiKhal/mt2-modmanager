use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GamePaths {
    pub install_dir: Option<PathBuf>,
    pub data_zip: Option<PathBuf>,
    pub profile_dir: Option<PathBuf>,
}

pub const APP_ID: &str = "486860";
const PREF_SUBDIR: [&str; 2] = ["VectorStorm", "MMORPG Tycoon 2"];


impl GamePaths {
    pub fn mod_dir(&self) -> Option<PathBuf> {
        self.profile_dir.as_ref().map(|p| p.join("mod"))
    }
    pub fn manager_dir(&self) -> Option<PathBuf> {
        self.profile_dir.as_ref().map(|p| p.join("mt2mm"))
    }
}


fn steam_roots() -> Vec<PathBuf> {
    let mut v = Vec::new();
    #[cfg(windows)]
    {
        use winreg::enums::HKEY_CURRENT_USER;
        if let Ok(k) = winreg::RegKey::predef(HKEY_CURRENT_USER).open_subkey("Software\\Valve\\Steam") {
            if let Ok(p) = k.get_value::<String, _>("SteamPath") {
                v.push(PathBuf::from(p));
            }
        }
        v.push(PathBuf::from(r"C:\Program Files (x86)\Steam"));
        v.push(PathBuf::from(r"C:\Program Files\Steam"));
    }
    if let Some(home) = dirs::home_dir() {
        #[cfg(target_os = "macos")]
        v.push(home.join("Library/Application Support/Steam"));
        #[cfg(all(unix, not(target_os = "macos")))]
        {
            v.push(home.join(".steam/steam"));
            v.push(home.join(".local/share/Steam"));
            v.push(home.join(".var/app/com.valvesoftware.Steam/.local/share/Steam"));
        }
        let _ = &home;
    }
    v.retain(|p| p.is_dir());
    v.dedup();
    v
}

fn vdf_values(text: &str, key: &str) -> Vec<String> {
    let mut out = Vec::new();
    let needle = format!("\"{key}\"");
    for line in text.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix(&needle) {
            let rest = rest.trim();
            if let Some(v) = rest.strip_prefix('"').and_then(|r| r.rfind('"').map(|i| &r[..i])) {
                out.push(v.replace("\\\\", "\\"));
            }
        }
    }
    out
}

pub fn steam_libraries() -> Vec<PathBuf> {
    let mut libs = Vec::new();
    for root in steam_roots() {
        libs.push(root.clone());
        let vdf = root.join("steamapps").join("libraryfolders.vdf");
        if let Ok(text) = std::fs::read_to_string(&vdf) {
            for p in vdf_values(&text, "path") {
                libs.push(PathBuf::from(p));
            }
        }
    }
    libs.retain(|p| p.join("steamapps").is_dir());
    libs.sort();
    libs.dedup();
    libs
}

pub fn find_install_dir() -> Option<PathBuf> {
    for lib in steam_libraries() {
        let acf = lib.join("steamapps").join(format!("appmanifest_{APP_ID}.acf"));
        if let Ok(text) = std::fs::read_to_string(&acf) {
            if let Some(dir) = vdf_values(&text, "installdir").into_iter().next() {
                let p = lib.join("steamapps").join("common").join(dir);
                if p.is_dir() {
                    return Some(p);
                }
            }
        }
    }
    None
}

pub fn find_data_zip(install_dir: &Path) -> Option<PathBuf> {
    walkdir::WalkDir::new(install_dir)
        .max_depth(6)
        .into_iter()
        .filter_map(|e| e.ok())
        .find(|e| e.file_type().is_file() && e.file_name() == "MMORPG.zip")
        .map(|e| e.into_path())
}


pub fn pref_roots() -> Vec<PathBuf> {
    let mut v = Vec::new();
    if let Some(d) = dirs::data_dir() {
        v.push(d.join(PREF_SUBDIR[0]).join(PREF_SUBDIR[1]));
    }
    // Windows build running under Proton on Linux
    for lib in steam_libraries() {
        v.push(
            ["steamapps", "compatdata", APP_ID, "pfx", "drive_c", "users", "steamuser", "AppData", "Roaming"]
                .iter()
                .fold(lib.clone(), |p, part| p.join(part))
                .join(PREF_SUBDIR[0])
                .join(PREF_SUBDIR[1]),
        );
    }
    v.retain(|p| p.is_dir());
    v
}

fn mtime(p: &Path) -> Option<std::time::SystemTime> {
    std::fs::metadata(p).and_then(|m| m.modified()).ok()
}

pub fn profiles_in(root: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(root)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.is_dir() && p.file_name().and_then(|n| n.to_str()).is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
        })
        .collect();
    v.sort();
    v
}

pub fn find_profile_dir() -> Option<PathBuf> {
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    for root in pref_roots() {
        for p in profiles_in(&root) {
            let t = mtime(&p.join("log.txt")).or_else(|| mtime(&p)).unwrap_or(std::time::UNIX_EPOCH);
            if best.as_ref().map_or(true, |(bt, _)| t > *bt) {
                best = Some((t, p));
            }
        }
    }
    best.map(|(_, p)| p)
}


pub fn detect() -> GamePaths {
    let install_dir = find_install_dir();
    let data_zip = install_dir.as_deref().and_then(find_data_zip);
    GamePaths { install_dir, data_zip, profile_dir: find_profile_dir() }
}


pub fn game_version(profile_dir: &Path) -> Option<String> {
    use std::io::{BufRead, BufReader};
    let f = std::fs::File::open(profile_dir.join("log.txt")).ok()?;
    for line in BufReader::new(f).lines().take(12).map_while(Result::ok) {
        if line.contains("main.cpp") {
            if let Some(v) = line.rsplit("-- ").next() {
                let v = v.trim();
                if v.chars().next().is_some_and(|c| c.is_ascii_digit()) && v.contains('.') {
                    return Some(v.to_string());
                }
            }
        }
    }
    None
}
