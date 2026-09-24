use std::path::{Component, Path};

pub const RECORD_EXTS: &[&str] = &[
    "txt", "cfg", "win", "def", "vrt", "costume", "variant", "conf", "defaults", "mat", "block",
    "prototype", "adv", "sequence", "vsprite", "tcolor", "tshape", "tstyle",
];


pub fn rel_string(p: &Path) -> String {
    p.components()
        .filter_map(|c| match c {
            Component::Normal(s) => Some(s.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

pub fn extension(rel: &str) -> String {
    let name = rel.rsplit('/').next().unwrap_or(rel);
    match name.rfind('.') {
        Some(i) if i > 0 => name[i + 1..].to_ascii_lowercase(),
        _ => String::new(),
    }
}


pub fn is_record_path(rel: &str) -> bool {
    let ext = extension(rel);
    // extension-less files under scenarios/ are step sequences
    RECORD_EXTS.contains(&ext.as_str()) || (ext.is_empty() && rel.starts_with("scenarios/"))
}

pub fn i18n_language(rel: &str) -> Option<&str> {
    let mut it = rel.split('/');
    match (it.next(), it.next(), it.next(), it.next()) {
        (Some("i18n"), Some(lang), Some(file), None) if file.to_ascii_lowercase().ends_with(".vrt") => Some(lang),
        _ => None,
    }
}


pub fn open_external(target: &std::ffi::OsStr) -> anyhow::Result<()> {
    #[cfg(windows)]
    let mut cmd = {
        let mut c = std::process::Command::new("explorer");
        c.arg(target);
        c
    };
    #[cfg(target_os = "macos")]
    let mut cmd = {
        let mut c = std::process::Command::new("open");
        c.arg(target);
        c
    };
    #[cfg(all(unix, not(target_os = "macos")))]
    let mut cmd = {
        let mut c = std::process::Command::new("xdg-open");
        c.arg(target);
        c
    };
    cmd.spawn()?;
    Ok(())
}
