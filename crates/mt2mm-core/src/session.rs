use crate::build::{self, BuildPlan};
use crate::deploy;
use crate::library::{unmanaged_mods, Library, LibraryMod};
use crate::paths::{self, GamePaths};
use crate::vanilla::Vanilla;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    pub install_dir: Option<PathBuf>,
    pub data: Option<PathBuf>,
    pub profile_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Status {
    pub paths: GamePaths,
    pub mod_dir: Option<PathBuf>,
    pub manager_dir: Option<PathBuf>,
    pub game_version: Option<String>,
    pub mods: Vec<LibraryMod>,
    pub unmanaged: Vec<PathBuf>,
    pub deployed: Vec<PathBuf>,
    pub deploy_state: String,
    pub profiles: Vec<String>,
    pub active_profile: String,
    pub deployed_mods: Vec<String>,
    pub problems: Vec<String>,
    pub game_update: Option<GameUpdate>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GameUpdate {
    pub data_time: u64,
    pub version: Option<String>,
}

pub struct Session {
    pub paths: GamePaths,
}


impl Config {
    pub fn path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("mt2mm").join("config.json"))
    }
    pub fn load() -> Config {
        Self::path().and_then(|p| std::fs::read_to_string(p).ok()).and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
    }
    pub fn save(&self) -> Result<()> {
        let p = Self::path().context("no config folder on this system")?;
        std::fs::create_dir_all(p.parent().unwrap())?;
        std::fs::write(p, serde_json::to_string_pretty(self)? + "\n")?;
        Ok(())
    }
}


pub fn fingerprint(mods: &[LibraryMod], data: Option<&Path>) -> String {
    let mut s = String::new();
    for line in mods.iter().filter(|m| m.enabled).filter_map(mod_fingerprint) {
        s += &line;
        s.push('\n');
    }
    s += &format!("data {}", data.map(newest).unwrap_or(0));
    s
}

fn mod_fingerprint(m: &LibraryMod) -> Option<String> {
    m.manifest.as_ref().map(|man| {
        let mut line = format!("{} {} {}", man.id, man.version, newest(&m.dir));
        if !m.config.is_empty() {
            line += &format!(" settings {}", crate::modconfig::hash(&m.settings.values));
        }
        line
    })
}

fn newest(p: &Path) -> u128 {
    walkdir::WalkDir::new(p)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter_map(|e| e.metadata().ok()?.modified().ok())
        .filter_map(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_millis())
        .max()
        .unwrap_or(0)
}


impl Session {
    pub fn open(overrides: &Config) -> Session {
        let mut p = paths::detect();
        let cfg = Config::load();
        for c in [&cfg, overrides] {
            if let Some(d) = &c.install_dir {
                p.install_dir = Some(d.clone());
                p.data_zip = paths::find_data_zip(d).or(p.data_zip.take());
            }
            if let Some(d) = &c.data {
                p.data_zip = Some(d.clone());
            }
            if let Some(d) = &c.profile_dir {
                p.profile_dir = Some(d.clone());
            }
        }
        Session { paths: p }
    }

    pub fn library(&self) -> Result<Library> {
        Ok(Library::new(&self.paths.manager_dir().context("game profile folder not found; run the game once, or set it in settings")?))
    }

    pub fn vanilla(&self) -> Result<Vanilla> {
        let p = self.paths.data_zip.as_ref().context("MMORPG.zip not found; set the game folder in settings")?;
        Vanilla::open(p)
    }

    pub fn status(&self) -> Status {
        let mut problems = Vec::new();
        if self.paths.install_dir.is_none() {
            problems.push("Game install folder not found.".into());
        }
        if self.paths.data_zip.is_none() {
            problems.push("MMORPG.zip not found.".into());
        }
        if self.paths.profile_dir.is_none() {
            problems.push("Game profile folder not found (run the game once).".into());
        }
        let mod_dir = self.paths.mod_dir();
        let state = self.library().map(|l| l.load_state()).unwrap_or_default();
        let mods = match self.library().and_then(|l| l.list()) {
            Ok(m) => m,
            Err(e) => {
                if self.paths.profile_dir.is_some() {
                    problems.push(format!("{e:#}"));
                }
                vec![]
            }
        };
        let deployed = mod_dir.as_deref().map(deploy::managed_folders).unwrap_or_default();
        let saved = if deployed.is_empty() {
            None
        } else {
            self.paths.manager_dir().and_then(|d| std::fs::read_to_string(d.join(deploy::DEPLOYED_FILE)).ok())
        };
        let deployed_mods = match &saved {
            Some(text) => mods
                .iter()
                .filter(|m| mod_fingerprint(m).is_some_and(|l| text.lines().any(|x| x == l)))
                .filter_map(|m| m.id().map(str::to_string))
                .collect(),
            None => vec![],
        };
        let deploy_state = if deployed.is_empty() {
            "none"
        } else {
            if saved.as_deref().map(str::trim_end) == Some(fingerprint(&mods, self.paths.data_zip.as_deref()).as_str()) {
                "current"
            } else {
                "outdated"
            }
        };
        let game_version = self.paths.profile_dir.as_deref().and_then(paths::game_version);
        let game_update = saved.as_deref().and_then(|text| self.game_update(text, game_version.as_deref()));
        Status {
            deploy_state: deploy_state.into(),
            profiles: state.profiles.keys().cloned().collect(),
            active_profile: state.active.clone(),
            deployed_mods,
            deployed,
            game_version,
            game_update,
            unmanaged: mod_dir.as_deref().map(unmanaged_mods).unwrap_or_default(),
            mod_dir,
            manager_dir: self.paths.manager_dir(),
            paths: self.paths.clone(),
            mods,
            problems,
        }
    }

    fn game_update(&self, saved: &str, game_version: Option<&str>) -> Option<GameUpdate> {
        let data = self.paths.data_zip.as_deref()?;
        let data_time = newest(data);
        let saved_line = saved.lines().rev().find(|line| line.starts_with("data "))?;
        if saved_line == "data 0" || saved_line == format!("data {data_time}") {
            return None;
        }
        let log_time = self.paths.profile_dir.as_deref().map(|dir| newest(&dir.join("log.txt"))).unwrap_or(0);
        // Until the game has run after the update, its log still names the old version.
        let version = if log_time > data_time { game_version.map(str::to_string) } else { None };
        Some(GameUpdate { data_time: data_time as u64, version })
    }

    pub fn plan(&self) -> Result<BuildPlan> {
        let lib = self.library()?;
        lib.normalize_state()?;
        let mods = lib.list()?;
        let vanilla = self.vanilla()?;
        let gv = self.paths.profile_dir.as_deref().and_then(paths::game_version);
        build::plan(&vanilla, &mods, gv)
    }

    pub fn deploy(&self) -> Result<BuildPlan> {
        let plan = self.plan()?;
        self.install(&plan)?;
        Ok(plan)
    }

    pub fn install(&self, plan: &BuildPlan) -> Result<()> {
        let mod_dir = self.paths.mod_dir().context("no profile folder")?;
        let mgr = self.paths.manager_dir().context("no profile folder")?;
        let fp = fingerprint(&self.library()?.list()?, self.paths.data_zip.as_deref());
        deploy::deploy(plan, &mod_dir, &mgr, fp)
    }

    pub fn clean(&self) -> Result<Vec<PathBuf>> {
        let mod_dir = self.paths.mod_dir().context("no profile folder")?;
        let removed = deploy::clean(&mod_dir)?;
        if let Some(d) = self.paths.manager_dir() {
            let _ = std::fs::remove_file(d.join(deploy::DEPLOYED_FILE));
        }
        Ok(removed)
    }

    pub fn launch_checked(&self) -> Result<(BuildPlan, bool)> {
        let plan = self.plan()?;
        if !plan.ok() {
            anyhow::bail!("the mods have errors (see the conflicts report); the game was not started");
        }
        let st = self.status();
        let in_sync = st.deploy_state == "current" || (st.deploy_state == "none" && !st.mods.iter().any(|m| m.enabled));
        if !in_sync {
            self.install(&plan)?;
        }
        self.launch()?;
        Ok((plan, !in_sync))
    }

    pub fn launch(&self) -> Result<()> {
        crate::util::open_external(std::ffi::OsStr::new(&format!("steam://rungameid/{}", paths::APP_ID)))
    }
}
