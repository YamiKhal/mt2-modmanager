use super::pe_exports::read_exports;
use std::collections::BTreeSet;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DllKind {
    Missing,
    LoaderProxy { version: String, exports: BTreeSet<String> },
    GameZlib { exports: BTreeSet<String> },
    Unknown { reason: String },
}

// Must match MT2LOADER_VERSION_MARKER and the export in mt2-loader/src/proxy.
const VERSION_MARKER: &[u8] = b"MT2LOADER_VERSION=";
const PROXY_EXPORT: &str = "mt2loader_proxy_version";
const CORE_EXPORT: &str = "mt2loader_start";
const ZLIB_EXPORTS: [&str; 3] = ["deflate", "inflate", "zlibVersion"];


fn marker_version(bytes: &[u8]) -> Option<String> {
    let start = bytes.windows(VERSION_MARKER.len()).position(|window| window == VERSION_MARKER)? + VERSION_MARKER.len();
    let length = bytes[start..].iter().take(32).position(|&byte| byte == 0)?;

    Some(String::from_utf8_lossy(&bytes[start..start + length]).into_owned())
}


pub fn identify_bytes(bytes: &[u8]) -> DllKind {
    let exports = match read_exports(bytes) {
        Ok(exports) => exports,
        Err(error) => return DllKind::Unknown { reason: error.to_string() },
    };

    let names: BTreeSet<String> = exports.iter().map(|export| export.name.clone()).collect();

    if names.contains(PROXY_EXPORT) {
        return DllKind::LoaderProxy { version: marker_version(bytes).unwrap_or_else(|| "unknown".into()), exports: names };
    }

    if ZLIB_EXPORTS.iter().all(|name| names.contains(*name)) {
        return DllKind::GameZlib { exports: names };
    }

    DllKind::Unknown { reason: "it's neither zlib nor the loader".into() }
}

pub fn identify_file(path: &Path) -> DllKind {
    match std::fs::read(path) {
        Ok(bytes) => identify_bytes(&bytes),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => DllKind::Missing,
        Err(error) => DllKind::Unknown { reason: format!("it can't be read ({error})") },
    }
}

pub fn is_loader_core(path: &Path) -> bool {
    let Ok(bytes) = std::fs::read(path) else {
        return false;
    };

    read_exports(&bytes).is_ok_and(|exports| exports.iter().any(|export| export.name == CORE_EXPORT))
}

// The proxy carries its own zlib, so it must offer every function the game's zlib1.dll does.
pub fn missing_exports(zlib_exports: &BTreeSet<String>, proxy_exports: &BTreeSet<String>) -> Vec<String> {
    zlib_exports.difference(proxy_exports).cloned().collect()
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_the_version_after_the_marker() {
        assert_eq!(marker_version(b"xxMT2LOADER_VERSION=0.1.0\0yy").as_deref(), Some("0.1.0"));
        assert_eq!(marker_version(b"no marker here"), None);
        assert_eq!(marker_version(b"MT2LOADER_VERSION=unterminated"), None);
    }

    #[test]
    fn unreadable_bytes_are_unknown() {
        assert!(matches!(identify_bytes(b"not a dll"), DllKind::Unknown { .. }));
    }

    #[test]
    fn lists_exports_the_proxy_lacks() {
        let zlib: BTreeSet<String> = ["deflate", "inflate", "gzopen"].map(String::from).into();
        let proxy: BTreeSet<String> = ["deflate", "inflate"].map(String::from).into();

        assert_eq!(missing_exports(&zlib, &proxy), vec!["gzopen".to_string()]);
        assert!(missing_exports(&proxy, &zlib).is_empty());
    }
}
