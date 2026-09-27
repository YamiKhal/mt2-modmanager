use super::identify::{identify_file, is_loader_core, DllKind};
use super::GameFolder;
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LoaderState {
    NotInstalled,
    Enabled,
    Disabled,
    UndoneBySteam,
    Broken,
}

#[derive(Debug, Clone, Serialize)]
pub struct LoaderStatus {
    pub state: LoaderState,
    pub version: Option<String>,
    pub problems: Vec<String>,
    pub log: Option<PathBuf>,
}


fn existing_log(folder: &GameFolder) -> Option<PathBuf> {
    Some(folder.log()).filter(|log| log.is_file())
}

fn installed_status(folder: &GameFolder, version: String) -> LoaderStatus {
    let mut problems = Vec::new();

    if !is_loader_core(&folder.core()) {
        problems.push("mt2loader\\mt2loader.dll is missing or damaged, so the game runs without the loader. Install it again.".to_string());
    }

    let state = if !problems.is_empty() {
        LoaderState::Broken
    } else if folder.disabled_flag().exists() {
        LoaderState::Disabled
    } else {
        LoaderState::Enabled
    };

    LoaderStatus { state, version: Some(version), problems, log: existing_log(folder) }
}


pub fn status(install_dir: &Path) -> LoaderStatus {
    let folder = GameFolder::new(install_dir);

    let broken = |problem: String| LoaderStatus { state: LoaderState::Broken, version: None, problems: vec![problem], log: None };

    match identify_file(&folder.proxy()) {
        DllKind::LoaderProxy { version, .. } => installed_status(&folder, version),
        DllKind::GameZlib { .. } if folder.loader_folder().exists() => LoaderStatus {
            state: LoaderState::UndoneBySteam,
            version: None,
            problems: vec!["Steam put the game's own zlib1.dll back (a game update or \"Verify integrity of game files\"), so the loader isn't running.".to_string()],
            log: existing_log(&folder),
        },
        DllKind::GameZlib { .. } => LoaderStatus { state: LoaderState::NotInstalled, version: None, problems: Vec::new(), log: None },
        DllKind::Missing => broken(
            "zlib1.dll is missing from the game folder, so the game can't start. Use Steam's \"Verify integrity of game files\".".to_string(),
        ),
        DllKind::Unknown { reason } => broken(format!(
            "The game folder's zlib1.dll isn't the game's or this loader's ({reason}). Another tool may have replaced it; the manager won't touch it."
        )),
    }
}
