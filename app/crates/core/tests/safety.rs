use std::fs;
use std::path::PathBuf;

use murating_core::slot::{self, SlotError};

fn scratch(tag: &str) -> PathBuf {
    let n = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let d = std::env::temp_dir().join(format!("murating-test-{tag}-{n}"));
    fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn writes_into_an_empty_directory() {
    let d = scratch("empty");
    let blob = slot::payload_only_bytes("MR OK 2304");

    let p = slot::write_slot(&d, &blob).expect("should write");
    assert_eq!(p.file_name().unwrap(), "MuRating.sav");
    assert_eq!(fs::read(&p).unwrap(), blob);

    fs::remove_dir_all(&d).ok();
}

#[test]
fn refuses_a_file_that_is_not_ours_and_changes_nothing() {
    let d = scratch("notours");
    let dest = d.join("MuRating.sav");
    let foreign = b"\x68\x3e\x70\x00 pretend this is a Tekken save".to_vec();
    fs::write(&dest, &foreign).unwrap();

    let err = slot::write_slot(&d, &slot::payload_only_bytes("x")).unwrap_err();
    assert!(matches!(err, SlotError::NotOurs(..)), "got {err:?}");

    assert_eq!(fs::read(&dest).unwrap(), foreign, "a refused write must not touch the file");

    fs::remove_dir_all(&d).ok();
}

#[test]
fn overwrites_our_own_previous_gvas_file() {
    let d = scratch("ours");
    let first = slot::payload_only_bytes("MR OK 1111");
    let second = slot::payload_only_bytes("MR OK 2222");

    slot::write_slot(&d, &first).unwrap();
    slot::write_slot(&d, &second).unwrap();

    assert_eq!(fs::read(d.join("MuRating.sav")).unwrap(), second);

    fs::remove_dir_all(&d).ok();
}

#[test]
fn leaves_no_temp_file_behind() {
    let d = scratch("tmp");
    slot::write_slot(&d, &slot::payload_only_bytes("MR OK 2304")).unwrap();

    let names: Vec<String> = fs::read_dir(&d)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
        .collect();
    assert_eq!(names, vec!["MuRating.sav".to_string()], "stray files: {names:?}");

    fs::remove_dir_all(&d).ok();
}

#[test]
fn a_refused_write_leaves_no_temp_file_either() {
    let d = scratch("refused-tmp");
    fs::write(d.join("MuRating.sav"), b"\x00\x01\x02\x03nope").unwrap();

    let _ = slot::write_slot(&d, &slot::payload_only_bytes("x"));

    let names: Vec<String> = fs::read_dir(&d)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
        .collect();
    assert_eq!(names, vec!["MuRating.sav".to_string()], "stray files: {names:?}");

    fs::remove_dir_all(&d).ok();
}

#[test]
fn neighbouring_files_are_never_touched() {
    let d = scratch("neighbours");
    let neighbours = [
        ("global1.sav", &b"h>\x00\x01 account data"[..]),
        ("ghost_1_11.sav", &b".WKK ghost data"[..]),
        ("storage001.sav", &b"[>p\x00 storage"[..]),
    ];
    for (n, b) in neighbours {
        fs::write(d.join(n), b).unwrap();
    }

    slot::write_slot(&d, &slot::payload_only_bytes("MR OK 2304")).unwrap();

    for (n, b) in neighbours {
        assert_eq!(fs::read(d.join(n)).unwrap(), b, "{n} was modified");
    }
    assert_eq!(fs::read_dir(&d).unwrap().count(), 4, "file count changed");

    fs::remove_dir_all(&d).ok();
}

#[test]
fn a_short_file_is_treated_as_not_ours() {
    let d = scratch("short");
    fs::write(d.join("MuRating.sav"), b"hi").unwrap();

    let err = slot::write_slot(&d, &slot::payload_only_bytes("x")).unwrap_err();
    assert!(matches!(err, SlotError::NotOurs(..)), "got {err:?}");

    fs::remove_dir_all(&d).ok();
}
