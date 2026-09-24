use crate::build::{BuildPlan, OutFile};
use crate::library::MARKER;
use anyhow::{bail, Context, Result};
use serde::Serialize;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;

#[derive(Serialize)]
struct Marker<'a> {
    tool: &'a str,
    version: &'a str,
    mod_id: Option<&'a str>,
}

// Sorts last in mod/, so the build wins over hand-installed mods.
pub const OUTPUT_FOLDER: &str = "zzzz_mm";
// One empty folder per mod, only so the in-game "Active Mods" window lists it.
pub const PLACEHOLDER_PREFIX: &str = "mm_";
const LEGACY_OUTPUT_FOLDER: &str = "zzzz_mt2mm";
const LEGACY_PLACEHOLDER_PREFIX: &str = "mt2mm_";
pub const DEPLOYED_FILE: &str = "deployed.txt";
// Measured with a 4657-file icon pack on an SSD: 1 writer 3.7 s, 2 writers 3.3 s, 4 or 8 writers
// 3.6 s (creating each file is the cost, not the data). Two also keeps a hard drive from seeking.
const MAX_WRITERS: usize = 2;


fn marker_json(ns: Option<&str>) -> String {
    serde_json::to_string_pretty(&Marker { tool: "mt2mm", version: env!("CARGO_PKG_VERSION"), mod_id: ns }).unwrap()
}

fn is_managed_name(name: &str) -> bool {
    [OUTPUT_FOLDER, LEGACY_OUTPUT_FOLDER].contains(&name)
        || name.starts_with(PLACEHOLDER_PREFIX)
        || name.starts_with(LEGACY_PLACEHOLDER_PREFIX)
}

pub fn managed_folders(mod_dir: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(mod_dir)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_dir() && !t.is_symlink()).unwrap_or(false))
        .map(|e| e.path())
        .filter(|p| p.join(MARKER).is_file() && p.file_name().and_then(|n| n.to_str()).is_some_and(is_managed_name))
        .collect();
    v.sort();
    v
}

fn guarded_remove(mod_dir: &Path, dir: &Path) -> Result<()> {
    let mod_canon = mod_dir.canonicalize()?;
    let canon = dir.canonicalize()?;
    let name = canon.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let meta = std::fs::symlink_metadata(dir)?;
    if canon.parent() != Some(mod_canon.as_path()) || !is_managed_name(name) || !canon.join(MARKER).is_file() || meta.file_type().is_symlink() {
        bail!("refusing to delete {} (not a folder created by the mod manager)", dir.display());
    }
    std::fs::remove_dir_all(&canon).with_context(|| format!("removing {}", canon.display()))
}

pub fn clean(mod_dir: &Path) -> Result<Vec<PathBuf>> {
    let found = managed_folders(mod_dir);
    for d in &found {
        guarded_remove(mod_dir, d)?;
    }
    Ok(found)
}


fn safe_join(root: &Path, rel: &str) -> Result<PathBuf> {
    let mut p = root.to_path_buf();
    for part in rel.split('/') {
        if part.is_empty() || part == "." || part == ".." || part.contains(['\\', ':']) {
            bail!("unsafe path in build output: {rel}");
        }
        p.push(part);
    }
    Ok(p)
}

fn write_files(files: &[OutFile], root: &Path) -> Result<()> {
    // Check every path before anything is written.
    let targets = files.iter().map(|f| safe_join(root, &f.rel)).collect::<Result<Vec<PathBuf>>>()?;

    let folders: BTreeSet<&Path> = targets.iter().filter_map(|p| p.parent()).collect();
    for folder in folders {
        std::fs::create_dir_all(folder)?;
    }

    let writers = std::thread::available_parallelism().map_or(1, |n| n.get()).clamp(1, MAX_WRITERS).min(files.len());
    let next = AtomicUsize::new(0);
    let failed = AtomicBool::new(false);
    let first_error: Mutex<Option<anyhow::Error>> = Mutex::new(None);

    std::thread::scope(|scope| {
        for _ in 0..writers {
            scope.spawn(|| {
                while !failed.load(Ordering::Relaxed) {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(file) = files.get(i) else { break };

                    if let Err(e) = file.write_to(&targets[i]).with_context(|| format!("writing {}", file.rel)) {
                        failed.store(true, Ordering::Relaxed);
                        first_error.lock().unwrap_or_else(|p| p.into_inner()).get_or_insert(e);
                    }
                }
            });
        }
    });

    match first_error.into_inner().unwrap_or_else(|p| p.into_inner()) {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

pub fn deploy(plan: &BuildPlan, mod_dir: &Path, manager_dir: &Path, fingerprint: String) -> Result<()> {
    if !plan.ok() {
        bail!("the build has errors; nothing was deployed");
    }
    std::fs::create_dir_all(mod_dir)?;
    std::fs::create_dir_all(manager_dir)?;
    let staging = manager_dir.join("staging");
    if staging.exists() {
        std::fs::remove_dir_all(&staging)?;
    }
    let out = staging.join(OUTPUT_FOLDER);
    std::fs::create_dir_all(&out)?;
    std::fs::write(out.join(MARKER), marker_json(None))?;
    write_files(&plan.files, &out)?;
    for ns in &plan.mods {
        let d = staging.join(format!("{PLACEHOLDER_PREFIX}{ns}"));
        std::fs::create_dir_all(&d)?;
        std::fs::write(d.join(MARKER), marker_json(Some(ns)))?;
    }

    // Refuse before changing anything if a folder we are about to create exists but is not ours.
    let managed = managed_folders(mod_dir);
    for e in std::fs::read_dir(&staging)? {
        let dest = mod_dir.join(e?.file_name());
        if dest.exists() && !managed.iter().any(|m| m.file_name() == dest.file_name()) {
            let _ = std::fs::remove_dir_all(&staging);
            bail!("{} already exists and was not created by the mod manager; rename or move it, then deploy again", dest.display());
        }
    }
    clean(mod_dir)?;
    for e in std::fs::read_dir(&staging)? {
        let e = e?;
        let dest = mod_dir.join(e.file_name());
        if std::fs::rename(e.path(), &dest).is_err() {
            crate::library::copy_dir(&e.path(), &dest)?;
        }
    }
    let _ = std::fs::remove_dir_all(&staging);
    std::fs::write(manager_dir.join("last_build.json"), serde_json::to_string_pretty(plan)? + "\n")?;
    std::fs::write(manager_dir.join(DEPLOYED_FILE), fingerprint + "\n")?;
    Ok(())
}

pub fn export(plan: &BuildPlan, dest: &Path) -> Result<()> {
    write_files(&plan.files, dest)?;
    std::fs::write(dest.join("mt2mm_report.json"), serde_json::to_string_pretty(plan)? + "\n")?;
    Ok(())
}
