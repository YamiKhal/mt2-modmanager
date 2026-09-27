use crate::library::LibraryMod;
use crate::loader::pe_exports::{read_exports, read_machine, MACHINE_X64};
use crate::manifest::{Manifest, MANIFEST_FILE};
use crate::modconfig::CONFIG_FILE;
use crate::merge::{Level, Report};
use anyhow::{Context, Result};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeKind {
    Plugin,
    Library,
}

#[derive(Debug, Clone, Serialize)]
pub struct NativeFile {
    pub mod_id: String,
    pub rel: String,
    pub kind: NativeKind,
    #[serde(skip)]
    pub source: PathBuf,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct LoaderPlan {
    pub needed_by: Vec<String>,
    pub files: Vec<NativeFile>,
    pub features: BTreeMap<String, Vec<String>>,
    pub game_builds: BTreeMap<String, Vec<String>>,
    // Deployed next to a plugin mod's DLLs, where the loader reads it.
    #[serde(skip)]
    pub manifests: BTreeMap<String, PathBuf>,
    // A plugin mod's settings, for its plugins to read: its config.json, and the player's values as settings.json.
    #[serde(skip)]
    pub settings: BTreeMap<String, PluginSettings>,
}

#[derive(Debug, Clone)]
pub struct PluginSettings {
    pub config: PathBuf,
    pub values: String,
}

pub const PLUGIN_SETTINGS_FILE: &str = "settings.json";

// Features the loader core offers to mods. Empty until the first one is built (LOADER.md §5).
pub const AVAILABLE_FEATURES: &[&str] = &[];
pub const PLUGIN_ENTRY: &str = "plugin_init";
const LOADER_FIELD: &str = "loader";


impl LoaderPlan {
    pub fn needed(&self) -> bool {
        !self.needed_by.is_empty()
    }

    pub fn plugins(&self) -> impl Iterator<Item = &NativeFile> {
        self.files.iter().filter(|file| file.kind == NativeKind::Plugin)
    }

    // The mods that bring native code: what the player approves before it runs.
    pub fn native_mods(&self) -> BTreeSet<String> {
        self.files.iter().map(|file| file.mod_id.clone()).collect()
    }
}


pub fn is_native_file(rel: &str) -> bool {
    rel.to_ascii_lowercase().ends_with(".dll")
}

fn normalize_declared(raw: &str) -> Option<String> {
    let rel = raw.replace('\\', "/");
    let parts: Vec<&str> = rel.split('/').collect();
    let unsafe_part = |part: &&str| part.is_empty() || *part == "." || *part == ".." || part.contains(':');

    if parts.iter().any(unsafe_part) {
        return None;
    }

    Some(rel)
}

fn is_build_name(build: &str) -> bool {
    !build.is_empty() && build.len() <= 32 && build.chars().all(|character| character.is_ascii_alphanumeric() || ".-_".contains(character))
}

fn available_features_text() -> String {
    if AVAILABLE_FEATURES.is_empty() {
        return "This loader has no features yet".to_string();
    }

    format!("Available: {}", AVAILABLE_FEATURES.join(", "))
}

pub fn native_files_in(mod_dir: &Path) -> Result<BTreeMap<String, PathBuf>> {
    let mut found = BTreeMap::new();

    for entry in walkdir::WalkDir::new(mod_dir).follow_links(false).into_iter().filter_map(|entry| entry.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }

        let rel = crate::util::rel_string(entry.path().strip_prefix(mod_dir)?);

        if is_native_file(&rel) {
            found.insert(rel, entry.path().to_path_buf());
        }
    }

    Ok(found)
}

fn declared_files(manifest: &Manifest, report: &mut Report) -> BTreeMap<String, (String, NativeKind)> {
    let id = manifest.id.as_str();
    let needs = manifest.loader.clone().unwrap_or_default();
    let mut declared = BTreeMap::new();

    for (list, kind, field) in [(&needs.plugins, NativeKind::Plugin, "loader.plugins"), (&needs.libraries, NativeKind::Library, "loader.libraries")] {
        for raw in list {
            let Some(rel) = normalize_declared(raw) else {
                report.push(Level::Error, id, MANIFEST_FILE, field, format!("'{raw}' isn't a path inside the mod (use a path like \"native/my_plugin.dll\")"));
                continue;
            };

            if declared.insert(rel.to_lowercase(), (rel, kind)).is_some() {
                report.push(Level::Error, id, MANIFEST_FILE, field, format!("'{raw}' is listed twice"));
            }
        }
    }

    declared
}

fn check_dll(id: &str, rel: &str, kind: NativeKind, source: &Path, report: &mut Report) -> Result<bool> {
    let bytes = std::fs::read(source).with_context(|| format!("reading {}", source.display()))?;

    let machine = match read_machine(&bytes) {
        Ok(machine) => machine,
        Err(error) => {
            report.push(Level::Error, id, rel, "", format!("isn't a Windows DLL ({error})"));

            return Ok(false);
        }
    };

    if machine != MACHINE_X64 {
        report.push(Level::Error, id, rel, "", "isn't a 64-bit DLL; the game is 64-bit, so it can't load it");

        return Ok(false);
    }

    let has_entry = read_exports(&bytes).is_ok_and(|exports| exports.iter().any(|export| export.name == PLUGIN_ENTRY));

    if kind == NativeKind::Plugin && !has_entry {
        report.push(
            Level::Error,
            id,
            rel,
            "",
            format!("has no {PLUGIN_ENTRY} function, so the loader can't start it. Plugins start from {PLUGIN_ENTRY} (see mt2loader.h); a DLL the plugin only uses goes in loader.libraries"),
        );

        return Ok(false);
    }

    Ok(true)
}


pub fn check_mod(library_mod: &LibraryMod, manifest: &Manifest, found: &BTreeMap<String, PathBuf>, report: &mut Report, plan: &mut LoaderPlan) -> Result<()> {
    let id = manifest.id.as_str();
    let needs = manifest.loader.clone().unwrap_or_default();
    let declared = declared_files(manifest, report);
    let mut files = Vec::new();

    for feature in &needs.features {
        if !AVAILABLE_FEATURES.contains(&feature.as_str()) {
            report.push(Level::Error, id, MANIFEST_FILE, "loader.features", format!("'{feature}' isn't a loader feature. {}", available_features_text()));
        }
    }

    for rel in found.keys().filter(|rel| !declared.contains_key(&rel.to_lowercase())) {
        report.push(
            Level::Error,
            id,
            rel,
            "",
            "is native code (a DLL) that manifest.json doesn't declare. List it in \"loader\": {\"plugins\": [...]} if the loader should start it, \
             or in \"libraries\" if a plugin uses it; otherwise delete it",
        );
    }

    for (key, (rel, kind)) in &declared {
        let Some((found_rel, source)) = found.iter().find(|(found_rel, _)| found_rel.to_lowercase() == *key) else {
            report.push(Level::Error, id, MANIFEST_FILE, LOADER_FIELD, format!("lists '{rel}', which isn't in the mod"));
            continue;
        };

        if check_dll(id, found_rel, *kind, source, report)? {
            files.push(NativeFile { mod_id: id.to_string(), rel: found_rel.clone(), kind: *kind, source: source.clone() });
        }
    }

    if !needs.libraries.is_empty() && needs.plugins.is_empty() {
        report.push(Level::Error, id, MANIFEST_FILE, "loader.libraries", "lists libraries but no plugin that uses them");
    }

    for build in needs.game_builds.iter().filter(|build| !is_build_name(build)) {
        report.push(Level::Error, id, MANIFEST_FILE, "loader.game_builds", format!("'{build}' isn't a game build name like \"0.30.7\""));
    }

    if !needs.plugins.is_empty() && needs.game_builds.is_empty() {
        report.push(
            Level::Error,
            id,
            MANIFEST_FILE,
            "loader.game_builds",
            "must list the game builds the plugins were tested on, like [\"0.30.7\"]. The loader starts plugins only on those builds",
        );
    }

    if !needs.plugins.is_empty() || !needs.features.is_empty() {
        plan.needed_by.push(id.to_string());
        plan.manifests.insert(id.to_string(), library_mod.dir.join(MANIFEST_FILE));
        plan.game_builds.insert(id.to_string(), needs.game_builds.clone());
    }

    if !needs.plugins.is_empty() && !library_mod.config.is_empty() {
        let values = serde_json::to_string_pretty(&library_mod.settings.values)? + "
";
        plan.settings.insert(id.to_string(), PluginSettings { config: library_mod.dir.join(CONFIG_FILE), values });
    }

    if !needs.features.is_empty() {
        plan.features.insert(id.to_string(), needs.features.clone());
    }

    plan.files.extend(files);

    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn declared_paths_stay_inside_the_mod() {
        assert_eq!(normalize_declared("native\\a.dll").as_deref(), Some("native/a.dll"));
        assert_eq!(normalize_declared("a.dll").as_deref(), Some("a.dll"));
        assert_eq!(normalize_declared("../a.dll"), None);
        assert_eq!(normalize_declared("/a.dll"), None);
        assert_eq!(normalize_declared("C:/a.dll"), None);
        assert_eq!(normalize_declared(""), None);
    }

    #[test]
    fn only_dlls_are_native() {
        assert!(is_native_file("native/Plugin.DLL"));
        assert!(!is_native_file("models/a.vmb"));
    }
}
