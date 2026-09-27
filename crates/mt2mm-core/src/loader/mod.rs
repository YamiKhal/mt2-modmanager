mod identify;
mod install;
mod overview;
pub(crate) mod pe_exports;
mod requirement;
mod status;

pub use install::{find_loader_files, install, remove, LoaderFiles};
pub use overview::{overview, LoaderOverview};
pub use requirement::missing_for;
pub use status::{status, LoaderState, LoaderStatus};

use std::path::{Path, PathBuf};

const GAME_EXE: &str = "MT2.exe";
const PROXY_FILE: &str = "zlib1.dll";
const LOADER_FOLDER: &str = "mt2loader";
const CORE_FILE: &str = "mt2loader.dll";
const BACKUP_FILE: &str = "zlib1.game.dll";
const DISABLED_FLAG: &str = "disabled";
const LOG_FILE: &str = "loader.log";
// Loader 0.1.0 forwarded to this copy of the game's zlib; removing the loader still tidies it up.
const LEGACY_ORIGINAL_FILE: &str = "zlib1_original.dll";
// Everything the loader or the manager may put in the loader folder; removal deletes only these.
const LOADER_FOLDER_FILES: [&str; 7] = [CORE_FILE, BACKUP_FILE, DISABLED_FLAG, LOG_FILE, "loader.previous.log", "session.txt", "plugins.txt"];
const TEMPORARY_SUFFIX: &str = ".mt2mm-new";


struct GameFolder {
    root: PathBuf,
}

impl GameFolder {
    fn new(install_dir: &Path) -> GameFolder {
        GameFolder { root: install_dir.to_path_buf() }
    }

    fn exe(&self) -> PathBuf {
        self.root.join(GAME_EXE)
    }

    fn proxy(&self) -> PathBuf {
        self.root.join(PROXY_FILE)
    }

    fn legacy_original(&self) -> PathBuf {
        self.root.join(LEGACY_ORIGINAL_FILE)
    }

    fn loader_folder(&self) -> PathBuf {
        self.root.join(LOADER_FOLDER)
    }

    fn core(&self) -> PathBuf {
        self.loader_folder().join(CORE_FILE)
    }

    fn backup(&self) -> PathBuf {
        self.loader_folder().join(BACKUP_FILE)
    }

    fn disabled_flag(&self) -> PathBuf {
        self.loader_folder().join(DISABLED_FLAG)
    }

    fn log(&self) -> PathBuf {
        self.loader_folder().join(LOG_FILE)
    }
}
