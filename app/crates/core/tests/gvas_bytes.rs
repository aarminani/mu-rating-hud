use murating_core::slot;

const FIXTURE: &[u8] = include_bytes!("fixtures/payload_only.sav");

#[test]
fn rust_matches_python_byte_for_byte() {
    let ours = slot::payload_only_bytes("MR OK 2304");

    assert_eq!(
        ours.len(),
        FIXTURE.len(),
        "length differs: rust {} vs python {}",
        ours.len(),
        FIXTURE.len()
    );

    if ours != FIXTURE {
        let at = ours
            .iter()
            .zip(FIXTURE.iter())
            .position(|(a, b)| a != b)
            .unwrap();
        let lo = at.saturating_sub(8);
        let hi = (at + 8).min(ours.len());
        panic!(
            "first difference at byte {at}\n  rust   {:02x?}\n  python {:02x?}",
            &ours[lo..hi],
            &FIXTURE[lo..hi]
        );
    }
}

#[test]
fn the_fixture_is_the_shape_we_think_it_is() {
    assert_eq!(&FIXTURE[0..4], b"GVAS");
    assert_eq!(FIXTURE.len(), 167);
    assert!(FIXTURE.ends_with(b"None\0"));
}

#[test]
fn size_field_is_int32_the_bug_that_shipped_an_empty_badge() {
    let b = slot::payload_only_bytes("MR OK 2304");
    let pos = find(&b, b"Payload\0").expect("Payload name in the archive");

    let after_name = pos + 8;
    let ty_len = i32::from_le_bytes(b[after_name..after_name + 4].try_into().unwrap()) as usize;
    assert_eq!(&b[after_name + 4..after_name + 4 + ty_len], b"StrProperty\0");

    let size_at = after_name + 4 + ty_len;
    let size = i32::from_le_bytes(b[size_at..size_at + 4].try_into().unwrap());
    assert_eq!(size, 15, "Size must be the FString byte count");

    let array_index = i32::from_le_bytes(b[size_at + 4..size_at + 8].try_into().unwrap());
    assert_eq!(array_index, 0, "ArrayIndex sits immediately after Size");
    assert_eq!(b[size_at + 8], 0, "HasPropertyGuid");
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}
