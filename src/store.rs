//! Where sheets live: files in the data dir, or localStorage in the browser. Every disk access of the app goes through here.
use std::path::{Path, PathBuf};

#[cfg(target_arch = "wasm32")]
pub(crate) use browser::*;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use disk::*;

#[cfg(not(target_arch = "wasm32"))]
mod disk {
    use super::*;

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

    /// Hand a file to the user: written in the current directory. Returns the lines to show.
    pub(crate) fn save_file(name: &str, bytes: &[u8]) -> Vec<String> {
        let path = std::env::current_dir().unwrap_or_default().join(name);
        match std::fs::write(&path, bytes) {
            Ok(()) => vec![name.to_string(), format!("in {}", path.parent().unwrap_or(&path).display())],
            Err(e) => vec![format!("Could not write {name}: {e}")],
        }
    }

    /// Every `*.json` in the data dir with its text.
    pub(crate) fn list_json() -> Vec<(PathBuf, String)> {
        std::fs::read_dir(data_dir()).into_iter().flatten().flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "json"))
            .filter_map(|p| std::fs::read_to_string(&p).ok().map(|s| (p, s)))
            .collect()
    }
}

/// The sheet's file name is its localStorage key under "dnd-tui/".
#[cfg(target_arch = "wasm32")]
mod browser {
    use super::*;

    pub(crate) fn data_dir() -> PathBuf { PathBuf::from("dnd-tui") }

    fn storage() -> Option<web_sys::Storage> { web_sys::window()?.local_storage().ok()? }

    fn key(p: &Path) -> String { format!("dnd-tui/{}", p.file_name().map(|n| n.to_string_lossy()).unwrap_or_default()) }

    pub(crate) fn exists(p: &Path) -> bool { storage().is_some_and(|s| s.get_item(&key(p)).ok().flatten().is_some()) }

    pub(crate) fn write(p: &Path, s: &str) { if let Some(st) = storage() { let _ = st.set_item(&key(p), s); } }

    pub(crate) fn remove(p: &Path) { if let Some(st) = storage() { let _ = st.remove_item(&key(p)); } }

    /// Hand a file to the user: the browser downloads it. Returns the lines to show.
    pub(crate) fn save_file(name: &str, bytes: &[u8]) -> Vec<String> {
        use web_sys::{js_sys, wasm_bindgen::JsCast};
        let go = || -> Option<()> {
            let parts = js_sys::Array::of1(&js_sys::Uint8Array::from(bytes));
            let blob = web_sys::Blob::new_with_u8_array_sequence(&parts).ok()?;
            let url = web_sys::Url::create_object_url_with_blob(&blob).ok()?;
            let doc = web_sys::window()?.document()?;
            let a: web_sys::HtmlAnchorElement = doc.create_element("a").ok()?.dyn_into().ok()?;
            a.set_href(&url);
            a.set_download(name);
            a.click();
            web_sys::Url::revoke_object_url(&url).ok()
        };
        match go() {
            Some(()) => vec![name.to_string(), "downloaded by the browser".to_string()],
            None => vec![format!("Could not save {name}")],
        }
    }

    pub(crate) fn list_json() -> Vec<(PathBuf, String)> {
        let Some(st) = storage() else { return vec![] };
        (0..st.length().unwrap_or(0)).filter_map(|i| st.key(i).ok().flatten())
            .filter_map(|k| {
                let name = k.strip_prefix("dnd-tui/")?.to_string();
                if !name.ends_with(".json") { return None; }
                Some((data_dir().join(name), st.get_item(&k).ok().flatten()?))
            })
            .collect()
    }
}
