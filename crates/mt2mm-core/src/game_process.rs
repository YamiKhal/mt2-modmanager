use std::ffi::OsStr;
use std::path::Path;
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

// Only MT2.exe is verified; the others are guesses for the macOS and Linux builds.
const GAME_NAMES: &[&str] = &["MT2.exe", "MT2", "MMORPG Tycoon 2"];


pub fn game_running(install_dir: Option<&Path>) -> bool {
    let mut system = System::new();
    system.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing());

    let candidates: Vec<Pid> = system
        .processes()
        .iter()
        .filter(|(_, process)| GAME_NAMES.iter().any(|name| process.name() == OsStr::new(name)))
        .map(|(pid, _)| *pid)
        .collect();

    if candidates.is_empty() {
        return false;
    }

    let Some(install_dir) = install_dir else {
        return true;
    };

    system.refresh_processes_specifics(
        ProcessesToUpdate::Some(&candidates),
        false,
        ProcessRefreshKind::nothing().with_exe(UpdateKind::OnlyIfNotSet),
    );

    candidates.iter().any(|pid| match system.process(*pid).and_then(|process| process.exe()) {
        Some(exe) => is_inside(exe, install_dir),
        // The path can't always be read (another user's process): trust the name.
        None => true,
    })
}

fn is_inside(path: &Path, folder: &Path) -> bool {
    // Steam's library file and the process list don't always agree on letter case or path form.
    let normalize = |p: &Path| p.canonicalize().unwrap_or_else(|_| p.to_path_buf()).to_string_lossy().to_lowercase();

    Path::new(&normalize(path)).starts_with(normalize(folder))
}
