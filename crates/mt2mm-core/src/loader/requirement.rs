use super::status::{status, LoaderState};
use crate::native::LoaderPlan;
use std::path::Path;


// Why the applied mods' plugins won't run, if they won't, worded to follow a mod's name. Only a warning:
// the mods' other files still work, and the loader can come from anywhere, not only from the manager.
pub fn missing_for(install_dir: Option<&Path>, plan: &LoaderPlan) -> Option<String> {
    if !plan.needed() {
        return None;
    }

    let Some(install_dir) = install_dir else {
        return Some("needs the MT2 Loader, and the game folder wasn't found to check for it".to_string());
    };

    let current = status(install_dir);

    match current.state {
        LoaderState::Enabled => None,
        LoaderState::NotInstalled | LoaderState::UndoneBySteam => {
            Some("needs the MT2 Loader, which isn't installed. Its plugins won't run until it is".to_string())
        }
        LoaderState::Disabled => Some("needs the MT2 Loader, which is disabled (mt2loader\\disabled exists)".to_string()),
        LoaderState::Broken => Some(format!("needs the MT2 Loader, which needs attention: {}", current.problems.join(" "))),
    }
}
