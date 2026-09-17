use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::gvas;

pub const CLASS: &str = "/Game/Mods/MuRating/SG_MuRating.SG_MuRating_C";

pub const SLOT_FILE: &str = "MuRating.sav";

const ALLOWED: [&str; 1] = [SLOT_FILE];

pub const SCHEMA_VERSION: i32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayMode {
    ShowBoth = 0,
    ShowMeOnly = 1,
}

#[derive(Debug, thiserror::Error)]
pub enum SlotError {
    #[error("{0:?} is not in the allowlist {ALLOWED:?}")]
    NotAllowed(String),
    #[error("{0} exists and was not written by this app (header {1:?}, not GVAS), nothing changed")]
    NotOurs(PathBuf, [u8; 4]),
    #[error("no Tekken 8 SaveGames directory at {0}")]
    NoSaveFolder(PathBuf),
    #[error("no account folder found under {0}")]
    NoAccount(PathBuf),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Default)]
pub struct SlotData {
    pub owner_tekken_id: String,
    pub written_at: i64,
    pub self_mr: i32,
    pub self_chara: i32,
    pub self_by_chara: Vec<(i32, i32)>,
    pub candidates: Vec<(String, i32)>,
    pub prev_mr: i32,
    pub change_id: i32,
    pub display_mode: DisplayMode,
    pub payload: String,
}

impl Default for DisplayMode {
    fn default() -> Self {
        DisplayMode::ShowBoth
    }
}

impl SlotData {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(512);
        gvas::write_header(&mut out, CLASS);

        gvas::prop_i32(&mut out, "SchemaVersion", SCHEMA_VERSION);
        gvas::prop_str(&mut out, "OwnerTekkenId", &self.owner_tekken_id);
        gvas::prop_i64(&mut out, "WrittenAt", self.written_at);
        gvas::prop_i32(&mut out, "SelfMr", self.self_mr);
        gvas::prop_i32(&mut out, "SelfChara", self.self_chara);
        gvas::prop_map_i32_i32(&mut out, "SelfByChara", &self.self_by_chara);
        gvas::prop_map_str_i32(&mut out, "Candidates", &self.candidates);
        gvas::prop_i32(&mut out, "PrevMr", self.prev_mr);
        gvas::prop_i32(&mut out, "ChangeId", self.change_id);
        gvas::prop_i32(&mut out, "DisplayMode", self.display_mode as i32);
        gvas::prop_str(&mut out, "Payload", &self.payload);

        gvas::write_terminator(&mut out);
        out
    }
}

pub fn payload_only_bytes(payload: &str) -> Vec<u8> {
    let mut out = Vec::new();
    gvas::write_header(&mut out, CLASS);
    gvas::prop_str(&mut out, "Payload", payload);
    gvas::write_terminator(&mut out);
    out
}

pub fn replay_bytes(payload: &str, mr: &[(String, String)], delta: &[(String, i32)]) -> Vec<u8> {
    let mut out = Vec::with_capacity(64 + mr.len() * 48);
    gvas::write_header(&mut out, CLASS);
    gvas::prop_str(&mut out, "Payload", payload);
    gvas::prop_map_str_str(&mut out, "ReplayMr", mr);
    gvas::prop_map_str_i32(&mut out, "ReplayDelta", delta);
    gvas::write_terminator(&mut out);
    out
}

pub fn write_slot(dir: &Path, blob: &[u8]) -> Result<PathBuf, SlotError> {
    if !ALLOWED.contains(&SLOT_FILE) {
        return Err(SlotError::NotAllowed(SLOT_FILE.to_string()));
    }

    let dest = dir.join(SLOT_FILE);

    if dest.exists() {
        let head = read_magic(&dest)?;
        if &head != b"GVAS" {
            return Err(SlotError::NotOurs(dest, head));
        }
    }

    let tmp = dir.join(format!("{SLOT_FILE}.tmp"));
    {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(blob)?;
        f.sync_all()?;
    }
    fs::rename(&tmp, &dest)?;

    Ok(dest)
}

fn read_magic(p: &Path) -> std::io::Result<[u8; 4]> {
    use std::io::Read;
    let mut f = fs::File::open(p)?;
    let mut head = [0u8; 4];
    let _ = f.read(&mut head)?;
    Ok(head)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payload_only_matches_the_python_shape() {
        let b = payload_only_bytes("MR OK 2304");
        assert_eq!(&b[0..4], b"GVAS");
        assert!(b.ends_with(b"None\0"));
    }

    #[test]
    fn full_schema_writes_every_field_name() {
        let d = SlotData {
            owner_tekken_id: "2yh7ByTerD8a".into(),
            self_mr: 2304,
            ..Default::default()
        };
        let b = d.to_bytes();
        for name in [
            "SchemaVersion",
            "OwnerTekkenId",
            "WrittenAt",
            "SelfMr",
            "SelfChara",
            "SelfByChara",
            "Candidates",
            "PrevMr",
            "ChangeId",
            "DisplayMode",
            "Payload",
        ] {
            assert!(
                b.windows(name.len()).any(|w| w == name.as_bytes()),
                "{name} missing from the archive"
            );
        }
    }

    #[test]
    fn display_mode_defaults_to_show_both_which_is_zero() {
        assert_eq!(DisplayMode::default() as i32, 0);
    }
}
