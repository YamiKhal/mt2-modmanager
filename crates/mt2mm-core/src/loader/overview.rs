use super::install::find_loader_files;
use super::status::{status, LoaderState, LoaderStatus};
use serde::Serialize;
use std::path::Path;

// What the manager shows about the loader: its state in the game folder, and the version the manager can install.
#[derive(Debug, Clone, Serialize)]
pub struct LoaderOverview {
    #[serde(flatten)]
    pub status: LoaderStatus,
    pub available: Option<String>,
    pub update_available: bool,
}


fn version_parts(version: &str) -> Vec<u64> {
    version.split('.').map(|part| part.parse().unwrap_or(0)).collect()
}

fn is_newer(candidate: &str, installed: &str) -> bool {
    version_parts(candidate) > version_parts(installed)
}


pub fn overview(install_dir: &Path, source: Option<&Path>) -> LoaderOverview {
    let status = status(install_dir);
    let available = find_loader_files(source).ok().map(|files| files.version);
    let installed = matches!(status.state, LoaderState::Enabled | LoaderState::Disabled);

    let update_available = match (&status.version, &available) {
        (Some(installed_version), Some(available_version)) => installed && is_newer(available_version, installed_version),
        _ => false,
    };

    LoaderOverview { status, available, update_available }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_versions_by_number() {
        assert!(is_newer("0.10.0", "0.9.3"));
        assert!(is_newer("0.2.0", "0.1.0"));
        assert!(!is_newer("0.1.0", "0.1.0"));
        assert!(!is_newer("0.1.0", "0.2.0"));
    }
}
