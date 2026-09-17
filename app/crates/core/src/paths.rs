use std::fs;
use std::path::{Path, PathBuf};

use crate::slot::SlotError;

pub fn saves_root() -> PathBuf {
    let local = std::env::var("LOCALAPPDATA").unwrap_or_default();
    Path::new(&local).join("TEKKEN 8").join("Saved").join("SaveGames")
}

pub fn slot_dir() -> PathBuf {
    saves_root()
}

pub fn active_account(root: &Path) -> Result<PathBuf, SlotError> {
    if !root.is_dir() {
        return Err(SlotError::NoSaveFolder(root.to_path_buf()));
    }

    let mut best: Option<(PathBuf, std::time::SystemTime)> = None;
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let marker = entry.path().join("global1.sav");
        let Ok(meta) = fs::metadata(&marker) else {
            continue;
        };
        let Ok(m) = meta.modified() else { continue };
        if best.as_ref().map_or(true, |(_, bm)| m > *bm) {
            best = Some((entry.path(), m));
        }
    }

    best.map(|(p, _)| p)
        .ok_or_else(|| SlotError::NoAccount(root.to_path_buf()))
}

pub fn account_id(dir: &Path) -> String {
    dir.file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_root_is_an_error_not_a_panic() {
        let r = active_account(Path::new(r"Z:\nope\does\not\exist"));
        assert!(matches!(r, Err(SlotError::NoSaveFolder(_))));
    }

    #[test]
    fn saves_root_ends_with_the_expected_tail() {
        let p = saves_root();
        let s = p.to_string_lossy().replace('\\', "/");
        assert!(s.ends_with("TEKKEN 8/Saved/SaveGames"), "got {s}");
    }
}
