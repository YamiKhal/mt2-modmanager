use anyhow::{Context, Result};
use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

pub enum Vanilla {
    // The archive stays open: opening it reads its whole directory (6000+ entries), which costs as
    // much as reading dozens of files.
    Zip { path: PathBuf, index: BTreeMap<String, usize>, archive: Mutex<zip::ZipArchive<std::fs::File>> },
    Dir { root: PathBuf, files: BTreeMap<String, PathBuf> },
}


impl Vanilla {
    pub fn open(path: &Path) -> Result<Self> {
        if path.is_dir() {
            let mut files = BTreeMap::new();
            for e in walkdir::WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
                if e.file_type().is_file() {
                    let rel = e.path().strip_prefix(path).unwrap();
                    files.insert(crate::util::rel_string(rel), e.path().to_path_buf());
                }
            }
            return Ok(Vanilla::Dir { root: path.to_path_buf(), files });
        }
        let f = std::fs::File::open(path).with_context(|| format!("opening {}", path.display()))?;
        let mut z = zip::ZipArchive::new(f).with_context(|| format!("reading {}", path.display()))?;
        let mut index = BTreeMap::new();
        for i in 0..z.len() {
            let e = z.by_index(i)?;
            if e.is_file() {
                index.insert(e.name().replace('\\', "/"), i);
            }
        }
        Ok(Vanilla::Zip { path: path.to_path_buf(), index, archive: Mutex::new(z) })
    }

    pub fn source(&self) -> &Path {
        match self {
            Vanilla::Zip { path, .. } => path,
            Vanilla::Dir { root, .. } => root,
        }
    }

    pub fn contains(&self, rel: &str) -> bool {
        match self {
            Vanilla::Zip { index, .. } => index.contains_key(rel),
            Vanilla::Dir { files, .. } => files.contains_key(rel),
        }
    }

    pub fn paths(&self) -> Vec<String> {
        match self {
            Vanilla::Zip { index, .. } => index.keys().cloned().collect(),
            Vanilla::Dir { files, .. } => files.keys().cloned().collect(),
        }
    }

    pub fn read(&self, rel: &str) -> Result<Option<Vec<u8>>> {
        match self {
            Vanilla::Zip { index, archive, .. } => {
                let Some(&i) = index.get(rel) else { return Ok(None) };
                let mut z = archive.lock().unwrap_or_else(PoisonError::into_inner);
                let mut e = z.by_index(i)?;
                let mut buf = Vec::with_capacity(e.size() as usize);
                e.read_to_end(&mut buf)?;
                Ok(Some(buf))
            }
            Vanilla::Dir { files, .. } => match files.get(rel) {
                Some(p) => Ok(Some(std::fs::read(p)?)),
                None => Ok(None),
            },
        }
    }
}
