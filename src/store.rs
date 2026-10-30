//! Where sheets live: files in the data dir. Every disk access of the app goes through here.
use std::path::{Path, PathBuf};

pub(crate) fn data_dir() -> PathBuf {
    let base = std::env::var_os("XDG_DATA_HOME").map(PathBuf::from)
        .or_else(|| std::env::var_os("LOCALAPPDATA").map(PathBuf::from)) // Windows has no HOME
        .unwrap_or_else(|| PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".local/share"));
    base.join("dnd-tui")
}

pub(crate) fn exists(p: &Path) -> bool { p.exists() }

pub(crate) fn write(p: &Path, s: &str) {
    let _ = std::fs::create_dir_all(data_dir());
    let _ = std::fs::write(p, s);
}

pub(crate) fn remove(p: &Path) { let _ = std::fs::remove_file(p); }

/// Every `*.json` in the data dir with its text.
pub(crate) fn list_json() -> Vec<(PathBuf, String)> {
    std::fs::read_dir(data_dir()).into_iter().flatten().flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .filter_map(|p| std::fs::read_to_string(&p).ok().map(|s| (p, s)))
        .collect()
}
