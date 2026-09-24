use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Manifest {
    #[serde(alias = "namespace")]
    pub id: String,
    pub name: String,
    #[serde(default = "default_version")]
    pub version: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub description: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub contributors: Vec<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub mod_page: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub support: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub donate: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub source: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub license: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub incompatible: Vec<String>,
    #[serde(default)]
    pub replace: Vec<String>,
    #[serde(default)]
    pub game_versions: Vec<String>,
}

pub const MANIFEST_FILE: &str = "manifest.json";
pub const ICON_FILE: &str = "icon.png";


fn default_version() -> String {
    "0.0.0".into()
}

pub fn validate_id(ns: &str) -> Result<()> {
    let ok_len = (2..=24).contains(&ns.len());
    let ok_chars = ns.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
    let ok_start = ns.chars().next().is_some_and(|c| c.is_ascii_lowercase());
    if !(ok_len && ok_chars && ok_start) {
        bail!("mod id '{ns}' must be 2-24 chars of a-z, 0-9, _ and start with a letter");
    }
    if ns.ends_with('_') || ns.contains("__") {
        bail!("mod id '{ns}' must not end with '_' or contain '__'");
    }
    Ok(())
}

impl Manifest {
    pub fn load(mod_dir: &Path) -> Result<Self> {
        let p = mod_dir.join(MANIFEST_FILE);
        let text = std::fs::read_to_string(&p).with_context(|| format!("reading {}", p.display()))?;
        let m: Manifest = serde_json::from_str(&text).with_context(|| format!("parsing {}", p.display()))?;
        validate_id(&m.id).with_context(|| format!("in {}", p.display()))?;
        if m.name.trim().is_empty() {
            bail!("{}: 'name' is empty", p.display());
        }
        Ok(m)
    }

    pub fn save(&self, mod_dir: &Path) -> Result<()> {
        let p = mod_dir.join(MANIFEST_FILE);
        std::fs::write(&p, serde_json::to_string_pretty(self)? + "\n")?;
        Ok(())
    }
}

pub fn suggest_id(name: &str) -> String {
    let mut s: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '_' })
        .collect();
    while s.contains("__") {
        s = s.replace("__", "_");
    }
    let mut s = s.trim_matches('_').to_string();
    if !s.chars().next().is_some_and(|c| c.is_ascii_lowercase()) {
        s = format!("m_{s}");
    }
    s.truncate(24);
    let s = s.trim_end_matches('_').to_string();
    if s.len() < 2 { "mod".into() } else { s }
}
