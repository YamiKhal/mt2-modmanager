use crate::manifest::{suggest_id, Manifest, ICON_FILE, MANIFEST_FILE};
use crate::modconfig::{self, ConfigOption, Resolved, SavedAll};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProfileEntry {
    pub id: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct Profile {
    #[serde(default)]
    pub mods: Vec<ProfileEntry>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub settings: SavedAll,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct State {
    pub active: String,
    pub profiles: BTreeMap<String, Profile>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LibraryMod {
    pub folder: String,
    pub dir: PathBuf,
    pub manifest: Option<Manifest>,
    pub error: Option<String>,
    pub applied: bool,
    pub enabled: bool,
    pub icon: bool,
    pub config: Vec<ConfigOption>,
    pub config_error: Option<String>,
    pub settings: Resolved,
}

#[derive(Debug, Clone, Serialize)]
pub struct Imported {
    pub source: PathBuf,
    pub id: Option<String>,
    pub updated: bool,
    pub error: Option<String>,
}

pub struct Library {
    pub root: PathBuf,
}

pub const MARKER: &str = ".mt2mm";
pub const DEFAULT_PROFILE: &str = "Default";


impl Default for State {
    fn default() -> Self {
        State { active: DEFAULT_PROFILE.into(), profiles: BTreeMap::from([(DEFAULT_PROFILE.into(), Profile::default())]) }
    }
}

impl State {
    // Also reads mt2mm 0.1's single list (`{"order": [...], "disabled": [...]}`) as the "Default" profile.
    fn from_json(text: &str) -> Option<State> {
        let v: serde_json::Value = serde_json::from_str(text).ok()?;
        if v.get("profiles").is_some() {
            return serde_json::from_value(v).ok();
        }
        let strs = |k: &str| -> Vec<String> {
            v.get(k).and_then(|a| a.as_array()).map(|a| a.iter().filter_map(|s| s.as_str().map(str::to_string)).collect()).unwrap_or_default()
        };
        let disabled = strs("disabled");
        let mods = strs("order").into_iter().map(|id| ProfileEntry { enabled: !disabled.contains(&id), id }).collect();
        let p = Profile { mods, ..Default::default() };
        Some(State { active: DEFAULT_PROFILE.into(), profiles: BTreeMap::from([(DEFAULT_PROFILE.into(), p)]) })
    }

    fn fix(&mut self) {
        if self.profiles.is_empty() {
            self.profiles.insert(DEFAULT_PROFILE.into(), Profile::default());
        }
        if !self.profiles.contains_key(&self.active) {
            self.active = self.profiles.keys().next().unwrap().clone();
        }
    }

    pub fn active_profile(&self) -> &Profile {
        &self.profiles[&self.active]
    }

    fn active_mut(&mut self) -> &mut Profile {
        self.profiles.get_mut(&self.active).unwrap()
    }
}


impl LibraryMod {
    pub fn id(&self) -> Option<&str> {
        self.manifest.as_ref().map(|m| m.id.as_str())
    }
    pub fn icon_path(&self) -> Option<PathBuf> {
        self.icon.then(|| self.dir.join(ICON_FILE))
    }
}


fn valid_profile_name(name: &str) -> Result<String> {
    let n = name.trim();
    if n.is_empty() || n.chars().count() > 40 || n.chars().any(|c| c.is_control()) {
        bail!("profile names must be 1-40 characters");
    }
    Ok(n.to_string())
}

impl Library {
    pub fn new(manager_dir: &Path) -> Self {
        Library { root: manager_dir.to_path_buf() }
    }
    pub fn mods_dir(&self) -> PathBuf {
        self.root.join("library")
    }
    fn state_path(&self) -> PathBuf {
        self.root.join("state.json")
    }

    pub fn load_state(&self) -> State {
        let mut st = std::fs::read_to_string(self.state_path()).ok().and_then(|s| State::from_json(&s)).unwrap_or_default();
        st.fix();
        // mt2mm 0.3.0 kept one set of settings for all profiles: give every profile a copy
        if let Some(old) = modconfig::load_legacy(&self.root) {
            for p in st.profiles.values_mut() {
                for (id, s) in &old {
                    p.settings.entry(id.clone()).or_insert_with(|| s.clone());
                }
            }
            let legacy = self.root.join(modconfig::LEGACY_SETTINGS_FILE);
            if self.save_state(&st).is_ok() {
                let _ = std::fs::rename(&legacy, legacy.with_extension("json.migrated"));
            }
        }
        st
    }

    pub fn save_state(&self, st: &State) -> Result<()> {
        std::fs::create_dir_all(&self.root)?;
        std::fs::write(self.state_path(), serde_json::to_string_pretty(st)? + "\n")?;
        Ok(())
    }

    pub fn list(&self) -> Result<Vec<LibraryMod>> {
        let st = self.load_state();
        let prof = st.active_profile();
        let saved = &prof.settings;
        let mut mods = Vec::new();
        let dir = self.mods_dir();
        if dir.is_dir() {
            for e in std::fs::read_dir(&dir)? {
                let e = e?;
                let folder = e.file_name().to_string_lossy().into_owned();
                if !e.file_type()?.is_dir() || folder.starts_with('.') {
                    continue;
                }
                let (manifest, error) = match Manifest::load(&e.path()) {
                    Ok(m) => (Some(m), None),
                    Err(err) => (None, Some(format!("{err:#}"))),
                };
                let entry = manifest.as_ref().and_then(|m| prof.mods.iter().find(|p| p.id == m.id));
                let (config, config_error) = match modconfig::load(&e.path()) {
                    Ok(c) => (c, None),
                    Err(err) => (vec![], Some(format!("{err:#}"))),
                };
                let settings = manifest
                    .as_ref()
                    .map(|m| modconfig::resolve(&config, saved.get(&m.id), &m.version))
                    .unwrap_or_default();
                mods.push(LibraryMod {
                    config,
                    config_error,
                    settings,
                    folder,
                    icon: e.path().join(ICON_FILE).is_file(),
                    dir: e.path(),
                    applied: entry.is_some(),
                    enabled: entry.is_some_and(|p| p.enabled),
                    manifest,
                    error,
                });
            }
        }
        let pos = |m: &LibraryMod| m.id().and_then(|id| prof.mods.iter().position(|p| p.id == id)).unwrap_or(usize::MAX);
        let name = |m: &LibraryMod| m.manifest.as_ref().map_or(m.folder.to_lowercase(), |x| x.name.to_lowercase());
        mods.sort_by(|a, b| pos(a).cmp(&pos(b)).then_with(|| name(a).cmp(&name(b))));
        Ok(mods)
    }

    pub fn set_settings(&self, id: &str, changes: &BTreeMap<String, serde_json::Value>, reset: &[String]) -> Result<()> {
        let m = self.list()?.into_iter().find(|m| m.id() == Some(id)).with_context(|| format!("no mod '{id}' in the library"))?;
        if let Some(e) = &m.config_error {
            bail!("{e}");
        }
        let man = m.manifest.as_ref().unwrap();
        let mut values: BTreeMap<String, serde_json::Value> =
            m.settings.values.into_iter().filter(|(k, _)| !reset.contains(k)).collect();
        for k in reset {
            if !m.config.iter().any(|o| &o.key == k) {
                bail!("the mod has no setting '{k}'");
            }
        }
        values.extend(changes.iter().map(|(k, v)| (k.clone(), v.clone())));
        let s = modconfig::to_saved(&m.config, &values, &man.version)?;
        let mut st = self.load_state();
        let all = &mut st.active_mut().settings;
        if s.values.is_empty() {
            all.remove(id);
        } else {
            all.insert(id.to_string(), s);
        }
        self.save_state(&st)
    }

    fn ids(&self) -> Result<Vec<String>> {
        Ok(self.list()?.iter().filter_map(|m| m.id().map(str::to_string)).collect())
    }

    pub fn normalize_state(&self) -> Result<State> {
        let ids = self.ids()?;
        let mut st = self.load_state();
        for p in st.profiles.values_mut() {
            p.mods.retain(|e| ids.contains(&e.id));
            let mut seen = Vec::new();
            p.mods.retain(|e| if seen.contains(&e.id) { false } else { seen.push(e.id.clone()); true });
        }
        self.save_state(&st)?;
        Ok(st)
    }

    fn edit_active(&self, f: impl FnOnce(&mut Profile, &[String]) -> Result<()>) -> Result<()> {
        let ids = self.ids()?;
        let mut st = self.normalize_state()?;
        f(st.active_mut(), &ids)?;
        self.save_state(&st)
    }

    pub fn set_enabled(&self, id: &str, enabled: bool) -> Result<()> {
        self.edit_active(|p, ids| {
            if !ids.iter().any(|i| i == id) {
                bail!("no mod with id '{id}' in the library");
            }
            match p.mods.iter_mut().find(|e| e.id == id) {
                Some(e) => e.enabled = enabled,
                None => p.mods.push(ProfileEntry { id: id.into(), enabled }),
            }
            Ok(())
        })
    }

    pub fn set_order(&self, order: &[String]) -> Result<()> {
        self.edit_active(|p, _| {
            let mut new: Vec<ProfileEntry> = order.iter().filter_map(|id| p.mods.iter().find(|e| &e.id == id).cloned()).collect();
            for e in &p.mods {
                if !new.iter().any(|n| n.id == e.id) {
                    new.push(e.clone());
                }
            }
            p.mods = new;
            Ok(())
        })
    }

    pub fn apply(&self, ids: &[String]) -> Result<()> {
        self.edit_active(|p, known| {
            for id in ids {
                if !known.contains(id) {
                    bail!("no mod with id '{id}' in the library");
                }
                if !p.mods.iter().any(|e| &e.id == id) {
                    p.mods.push(ProfileEntry { id: id.clone(), enabled: true });
                }
            }
            Ok(())
        })
    }

    pub fn unapply(&self, ids: &[String]) -> Result<()> {
        self.edit_active(|p, _| {
            p.mods.retain(|e| !ids.contains(&e.id));
            Ok(())
        })
    }

    pub fn create_profile(&self, name: &str, copy_from: Option<&str>) -> Result<String> {
        let name = valid_profile_name(name)?;
        let mut st = self.normalize_state()?;
        if st.profiles.contains_key(&name) {
            bail!("a profile named '{name}' already exists");
        }
        let p = match copy_from {
            Some(src) => st.profiles.get(src).cloned().with_context(|| format!("no profile named '{src}'"))?,
            None => Profile::default(),
        };
        st.profiles.insert(name.clone(), p);
        st.active = name.clone();
        self.save_state(&st)?;
        Ok(name)
    }

    pub fn switch_profile(&self, name: &str) -> Result<()> {
        let mut st = self.normalize_state()?;
        if !st.profiles.contains_key(name) {
            bail!("no profile named '{name}'");
        }
        st.active = name.into();
        self.save_state(&st)
    }

    pub fn rename_profile(&self, old: &str, new: &str) -> Result<()> {
        let new = valid_profile_name(new)?;
        let mut st = self.normalize_state()?;
        if old == new {
            return Ok(());
        }
        if st.profiles.contains_key(&new) {
            bail!("a profile named '{new}' already exists");
        }
        let p = st.profiles.remove(old).with_context(|| format!("no profile named '{old}'"))?;
        st.profiles.insert(new.clone(), p);
        if st.active == old {
            st.active = new;
        }
        self.save_state(&st)
    }

    pub fn delete_profile(&self, name: &str) -> Result<()> {
        let mut st = self.normalize_state()?;
        if st.profiles.len() <= 1 {
            bail!("the last profile can't be deleted");
        }
        st.profiles.remove(name).with_context(|| format!("no profile named '{name}'"))?;
        st.fix();
        self.save_state(&st)
    }

    pub fn import(&self, src: &Path, move_folder: bool) -> Result<Imported> {
        let src = if src.file_name().is_some_and(|n| n == MANIFEST_FILE) && src.is_file() {
            src.parent().context("manifest.json has no folder")?
        } else {
            src
        };
        if src.is_file() && is_zip(src) {
            let tmp = self.root.join("tmp_import");
            if tmp.exists() {
                std::fs::remove_dir_all(&tmp)?;
            }
            std::fs::create_dir_all(&tmp)?;
            let res = extract_zip(src, &tmp).and_then(|_| {
                let inner = find_mod_root(&tmp)?;
                let stem = src.file_stem().unwrap().to_string_lossy().into_owned();
                self.import_dir(&inner, &stem, true)
            });
            let _ = std::fs::remove_dir_all(&tmp);
            let (id, updated) = res?;
            return Ok(Imported { source: src.into(), id: Some(id), updated, error: None });
        }
        if !src.is_dir() {
            bail!("{} is not a mod folder, .zip or manifest.json", src.display());
        }
        let name = src.file_name().context("no folder name")?.to_string_lossy().into_owned();
        let (id, updated) = self.import_dir(src, &name, move_folder)?;
        Ok(Imported { source: src.into(), id: Some(id), updated, error: None })
    }

    pub fn import_all(&self, dir: &Path) -> Result<Vec<Imported>> {
        if dir.join(MANIFEST_FILE).is_file() {
            return Ok(vec![self.import(dir, false)?]);
        }
        let mut cands: Vec<PathBuf> = std::fs::read_dir(dir)
            .with_context(|| format!("reading {}", dir.display()))?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| (p.is_dir() && p.join(MANIFEST_FILE).is_file()) || (p.is_file() && is_zip(p) && zip_has_manifest(p)))
            .collect();
        cands.sort();
        Ok(cands
            .into_iter()
            .map(|p| match self.import(&p, false) {
                Ok(r) => r,
                Err(e) => Imported { source: p, id: None, updated: false, error: Some(format!("{e:#}")) },
            })
            .collect())
    }

    fn import_dir(&self, src: &Path, display_name: &str, move_folder: bool) -> Result<(String, bool)> {
        let existing = self.list()?;
        let manifest = match Manifest::load(src) {
            Ok(m) => m,
            Err(_) if !src.join(MANIFEST_FILE).exists() => {
                let mut id = suggest_id(display_name);
                let base = id.clone();
                let mut n = 2;
                while existing.iter().any(|m| m.id() == Some(id.as_str())) {
                    id = format!("{}_{n}", &base[..base.len().min(21)]);
                    n += 1;
                }
                Manifest {
                    id,
                    name: display_name.to_string(),
                    version: "0.0.0".into(),
                    author: String::new(),
                    description: "Added without a manifest.json; the mod manager generated one.".into(),
                    ..Default::default()
                }
            }
            Err(e) => return Err(e),
        };
        let lib = self.mods_dir();
        std::fs::create_dir_all(&lib)?;
        let old = existing.iter().find(|m| m.id() == Some(manifest.id.as_str()));
        if let Some(old) = old {
            if old.dir.canonicalize()? == src.canonicalize()? {
                bail!("'{}' is already this library's copy", src.display());
            }
        }
        // stage next to the library, then swap, so a failed copy never loses the old version
        let staged = lib.join(format!(".incoming_{}", manifest.id));
        if staged.exists() {
            std::fs::remove_dir_all(&staged)?;
        }
        let moved = move_folder && std::fs::rename(src, &staged).is_ok();
        if !moved {
            if let Err(e) = copy_dir(src, &staged) {
                let _ = std::fs::remove_dir_all(&staged);
                return Err(e);
            }
        }
        let _ = std::fs::remove_file(staged.join(MARKER));
        manifest.save(&staged)?;
        if let Some(old) = old {
            self.remove_dir_in_library(&old.dir)?;
        }
        let dest = lib.join(&manifest.id);
        if dest.exists() {
            // a folder named like the id but holding another (or a broken) mod
            self.remove_dir_in_library(&dest)?;
        }
        std::fs::rename(&staged, &dest)?;
        if old.is_none() {
            self.apply(&[manifest.id.clone()])?;
        } else {
            self.normalize_state()?;
        }
        Ok((manifest.id, old.is_some()))
    }

    fn remove_dir_in_library(&self, dir: &Path) -> Result<()> {
        let lib = self.mods_dir().canonicalize()?;
        let target = dir.canonicalize()?;
        if target.parent() != Some(lib.as_path()) {
            bail!("refusing to delete {} (outside the library)", target.display());
        }
        std::fs::remove_dir_all(&target)?;
        Ok(())
    }

    pub fn remove(&self, id: &str) -> Result<()> {
        let mods = self.list()?;
        let m = mods
            .iter()
            .find(|m| m.id() == Some(id))
            .or_else(|| mods.iter().find(|m| m.id().is_none() && m.folder == id))
            .context("no such mod")?;
        self.remove_dir_in_library(&m.dir)?;
        self.normalize_state()?;
        Ok(())
    }
}


fn is_zip(p: &Path) -> bool {
    p.extension().is_some_and(|e| e.eq_ignore_ascii_case("zip"))
}

fn zip_has_manifest(p: &Path) -> bool {
    std::fs::File::open(p)
        .ok()
        .and_then(|f| zip::ZipArchive::new(f).ok())
        .is_some_and(|z| z.file_names().any(|n| n == MANIFEST_FILE || n.ends_with(&format!("/{MANIFEST_FILE}"))))
}

pub fn copy_dir(src: &Path, dest: &Path) -> Result<()> {
    for e in walkdir::WalkDir::new(src).follow_links(false) {
        let e = e?;
        let rel = e.path().strip_prefix(src)?;
        let to = dest.join(rel);
        if e.file_type().is_dir() {
            std::fs::create_dir_all(&to)?;
        } else if e.file_type().is_file() {
            if let Some(p) = to.parent() {
                std::fs::create_dir_all(p)?;
            }
            std::fs::copy(e.path(), &to).with_context(|| format!("copying {}", e.path().display()))?;
        }
    }
    Ok(())
}

fn extract_zip(zip_path: &Path, dest: &Path) -> Result<()> {
    let f = std::fs::File::open(zip_path)?;
    let mut z = zip::ZipArchive::new(f).with_context(|| format!("{} is not a valid .zip", zip_path.display()))?;
    for i in 0..z.len() {
        let mut e = z.by_index(i)?;
        let Some(rel) = e.enclosed_name() else { continue }; // rejects ../ paths
        let out = dest.join(rel);
        if e.is_dir() {
            std::fs::create_dir_all(&out)?;
        } else {
            if let Some(p) = out.parent() {
                std::fs::create_dir_all(p)?;
            }
            let mut w = std::fs::File::create(&out)?;
            std::io::copy(&mut e, &mut w)?;
        }
    }
    Ok(())
}

fn find_mod_root(dir: &Path) -> Result<PathBuf> {
    if dir.join(MANIFEST_FILE).is_file() {
        return Ok(dir.to_path_buf());
    }
    for e in walkdir::WalkDir::new(dir).max_depth(3) {
        let e = e?;
        if e.file_type().is_file() && e.file_name() == MANIFEST_FILE {
            return Ok(e.path().parent().unwrap().to_path_buf());
        }
    }
    let entries: Vec<_> = std::fs::read_dir(dir)?.filter_map(|e| e.ok()).collect();
    if entries.len() == 1 && entries[0].path().is_dir() {
        return Ok(entries[0].path());
    }
    Ok(dir.to_path_buf())
}


pub fn unmanaged_mods(mod_dir: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(mod_dir)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_dir() && !p.join(MARKER).exists())
        .collect();
    v.sort();
    v
}


#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("mt2mm_libtest_{name}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn make_mod(root: &Path, folder: &str, id: &str, version: &str) -> PathBuf {
        let d = root.join(folder);
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join(MANIFEST_FILE), format!(r#"{{"id":"{id}","name":"{id} mod","version":"{version}"}}"#)).unwrap();
        d
    }

    #[test]
    fn legacy_state_becomes_default_profile() {
        let st = State::from_json(r#"{"order":["a","b"],"disabled":["b"]}"#).unwrap();
        assert_eq!(st.active, DEFAULT_PROFILE);
        assert_eq!(
            st.active_profile().mods,
            vec![ProfileEntry { id: "a".into(), enabled: true }, ProfileEntry { id: "b".into(), enabled: false }]
        );
    }

    #[test]
    fn profiles_switch_applied_mods() {
        let root = tmp("profiles");
        let src = root.join("src");
        let lib = Library::new(&root.join("mgr"));
        lib.import(&make_mod(&src, "a", "alpha", "1"), false).unwrap();
        lib.import(&make_mod(&src, "b", "beta", "1"), false).unwrap();
        let on = |l: &Library| l.list().unwrap().iter().filter(|m| m.enabled).map(|m| m.id().unwrap().to_string()).collect::<Vec<_>>();
        assert_eq!(on(&lib), ["alpha", "beta"]);

        lib.create_profile("Only beta", Some(DEFAULT_PROFILE)).unwrap();
        lib.unapply(&["alpha".into()]).unwrap();
        assert_eq!(on(&lib), ["beta"]);
        lib.switch_profile(DEFAULT_PROFILE).unwrap();
        assert_eq!(on(&lib), ["alpha", "beta"]);
        lib.set_order(&["beta".into(), "alpha".into()]).unwrap();
        lib.set_enabled("alpha", false).unwrap();
        assert_eq!(on(&lib), ["beta"]);
        assert_eq!(lib.list().unwrap()[1].id(), Some("alpha"));

        lib.rename_profile("Only beta", "B").unwrap();
        lib.delete_profile("B").unwrap();
        assert!(lib.delete_profile(DEFAULT_PROFILE).is_err());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn settings_are_per_profile() {
        let root = tmp("settings");
        let src = make_mod(&root, "src_a", "alpha", "1.0.0");
        std::fs::write(src.join("config.json"), r#"[{"key":"price","type":"int","default":10}]"#).unwrap();
        let lib = Library::new(&root.join("mgr"));
        lib.import(&src, false).unwrap();
        let price = |lib: &Library| lib.list().unwrap()[0].settings.values["price"].clone();
        let set = |v: i64| [("price".to_string(), serde_json::json!(v))].into_iter().collect();
        lib.set_settings("alpha", &set(20), &[]).unwrap();
        lib.create_profile("Copy", Some(DEFAULT_PROFILE)).unwrap();
        assert_eq!(price(&lib), serde_json::json!(20), "a copied profile keeps the settings");
        lib.set_settings("alpha", &set(30), &[]).unwrap();
        lib.create_profile("Empty", None).unwrap();
        assert_eq!(price(&lib), serde_json::json!(10), "a new profile starts with the defaults");
        lib.switch_profile(DEFAULT_PROFILE).unwrap();
        assert_eq!(price(&lib), serde_json::json!(20));
        lib.switch_profile("Copy").unwrap();
        assert_eq!(price(&lib), serde_json::json!(30));

        // 0.3.0's shared file is copied into every profile that has nothing for that mod
        std::fs::write(root.join("mgr").join("mod_settings.json"), r#"{"alpha":{"version":"1.0.0","values":{"price":40}}}"#).unwrap();
        assert_eq!(price(&lib), serde_json::json!(30));
        lib.switch_profile("Empty").unwrap();
        assert_eq!(price(&lib), serde_json::json!(40));
        assert!(!root.join("mgr").join("mod_settings.json").exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn reimport_updates_and_folder_scan() {
        let root = tmp("scan");
        let src = root.join("src");
        let lib = Library::new(&root.join("mgr"));
        make_mod(&src, "one", "one", "1.0");
        make_mod(&src, "two", "two", "1.0");
        std::fs::create_dir_all(src.join("not_a_mod")).unwrap();
        let r = lib.import_all(&src).unwrap();
        assert_eq!(r.iter().filter_map(|i| i.id.clone()).collect::<Vec<_>>(), ["one", "two"]);

        lib.set_enabled("one", false).unwrap();
        let newer = make_mod(&root.join("src2"), "one_v2", "one", "2.0");
        let r = lib.import(&newer.join(MANIFEST_FILE), false).unwrap();
        assert!(r.updated);
        let mods = lib.list().unwrap();
        let one = mods.iter().find(|m| m.id() == Some("one")).unwrap();
        assert_eq!(one.manifest.as_ref().unwrap().version, "2.0");
        assert!(one.applied && !one.enabled, "update keeps the profile entry");
        let _ = std::fs::remove_dir_all(&root);
    }
}
