pub fn write_fstring(out: &mut Vec<u8>, s: &str) {
    if s.is_empty() {
        out.extend_from_slice(&0i32.to_le_bytes());
        return;
    }
    if s.is_ascii() {
        let b = s.as_bytes();
        out.extend_from_slice(&((b.len() + 1) as i32).to_le_bytes());
        out.extend_from_slice(b);
        out.push(0);
    } else {
        let units: Vec<u16> = s.encode_utf16().collect();
        let len = -((units.len() as i64 + 1) as i64) as i32;
        out.extend_from_slice(&len.to_le_bytes());
        for u in units {
            out.extend_from_slice(&u.to_le_bytes());
        }
        out.extend_from_slice(&[0, 0]);
    }
}

fn fstring_vec(s: &str) -> Vec<u8> {
    let mut v = Vec::new();
    write_fstring(&mut v, s);
    v
}

pub fn write_header(out: &mut Vec<u8>, class: &str) {
    out.extend_from_slice(b"GVAS");
    out.extend_from_slice(&3i32.to_le_bytes());
    out.extend_from_slice(&522i32.to_le_bytes());
    out.extend_from_slice(&1009i32.to_le_bytes());
    out.extend_from_slice(&5u16.to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    write_fstring(out, "++UE5+Release-5.2");
    out.extend_from_slice(&3i32.to_le_bytes());
    out.extend_from_slice(&0i32.to_le_bytes());
    write_fstring(out, class);
}

fn tagged(out: &mut Vec<u8>, name: &str, ty: &str, extra: &[u8], body: &[u8]) {
    write_fstring(out, name);
    write_fstring(out, ty);
    out.extend_from_slice(&(body.len() as i32).to_le_bytes());
    out.extend_from_slice(&0i32.to_le_bytes());
    out.extend_from_slice(extra);
    out.push(0u8);
    out.extend_from_slice(body);
}

pub fn prop_str(out: &mut Vec<u8>, name: &str, value: &str) {
    tagged(out, name, "StrProperty", &[], &fstring_vec(value));
}

pub fn prop_i32(out: &mut Vec<u8>, name: &str, value: i32) {
    tagged(out, name, "IntProperty", &[], &value.to_le_bytes());
}

pub fn prop_i64(out: &mut Vec<u8>, name: &str, value: i64) {
    tagged(out, name, "Int64Property", &[], &value.to_le_bytes());
}

pub fn prop_map_i32_i32(out: &mut Vec<u8>, name: &str, entries: &[(i32, i32)]) {
    let mut extra = Vec::new();
    write_fstring(&mut extra, "IntProperty");
    write_fstring(&mut extra, "IntProperty");
    let mut body = map_body_head(entries.len());
    for (k, v) in entries {
        body.extend_from_slice(&k.to_le_bytes());
        body.extend_from_slice(&v.to_le_bytes());
    }
    tagged(out, name, "MapProperty", &extra, &body);
}

pub fn prop_map_str_i32(out: &mut Vec<u8>, name: &str, entries: &[(String, i32)]) {
    let mut extra = Vec::new();
    write_fstring(&mut extra, "StrProperty");
    write_fstring(&mut extra, "IntProperty");
    let mut body = map_body_head(entries.len());
    for (k, v) in entries {
        write_fstring(&mut body, k);
        body.extend_from_slice(&v.to_le_bytes());
    }
    tagged(out, name, "MapProperty", &extra, &body);
}

pub fn prop_map_str_str(out: &mut Vec<u8>, name: &str, entries: &[(String, String)]) {
    let mut extra = Vec::new();
    write_fstring(&mut extra, "StrProperty");
    write_fstring(&mut extra, "StrProperty");
    let mut body = map_body_head(entries.len());
    for (k, v) in entries {
        write_fstring(&mut body, k);
        write_fstring(&mut body, v);
    }
    tagged(out, name, "MapProperty", &extra, &body);
}

fn map_body_head(n: usize) -> Vec<u8> {
    let mut b = Vec::new();
    b.extend_from_slice(&0i32.to_le_bytes());
    b.extend_from_slice(&(n as i32).to_le_bytes());
    b
}

pub fn write_terminator(out: &mut Vec<u8>) {
    write_fstring(out, "None");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_fstring_is_four_zero_bytes() {
        let mut v = Vec::new();
        write_fstring(&mut v, "");
        assert_eq!(v, vec![0, 0, 0, 0]);
    }

    #[test]
    fn ascii_fstring_length_includes_the_nul() {
        let mut v = Vec::new();
        write_fstring(&mut v, "None");
        assert_eq!(&v[0..4], &5i32.to_le_bytes());
        assert_eq!(&v[4..], b"None\0");
    }

    #[test]
    fn non_ascii_fstring_is_negative_length_utf16() {
        let mut v = Vec::new();
        write_fstring(&mut v, "\u{200b}A");
        let len = i32::from_le_bytes(v[0..4].try_into().unwrap());
        assert_eq!(len, -3, "2 code units + terminator, negated");
        assert_eq!(&v[4..], &[0x0b, 0x20, 0x41, 0x00, 0x00, 0x00]);
    }

    #[test]
    fn surrogate_pair_counts_as_two_units() {
        let mut v = Vec::new();
        write_fstring(&mut v, "\u{1F600}");
        let len = i32::from_le_bytes(v[0..4].try_into().unwrap());
        assert_eq!(len, -3);
    }

    #[test]
    fn size_field_is_int32_and_matches_the_body() {
        let mut v = Vec::new();
        prop_i32(&mut v, "SelfMr", 2304);
        let mut off = 0;
        for _ in 0..2 {
            let n = i32::from_le_bytes(v[off..off + 4].try_into().unwrap());
            off += 4 + n as usize;
        }
        assert_eq!(i32::from_le_bytes(v[off..off + 4].try_into().unwrap()), 4);
    }

    #[test]
    fn string_map_size_covers_both_fstrings_per_entry() {
        let mut v = Vec::new();
        let entries = vec![
            ("MRmani|407811".to_string(), "2111 MR".to_string()),
            ("\u{044e}\u{0442}|538948".to_string(), "2301 MR".to_string()),
        ];
        prop_map_str_str(&mut v, "ReplayMr", &entries);

        let mut body = Vec::new();
        body.extend_from_slice(&0i32.to_le_bytes());
        body.extend_from_slice(&2i32.to_le_bytes());
        for (k, val) in &entries {
            write_fstring(&mut body, k);
            write_fstring(&mut body, val);
        }
        assert!(v.ends_with(&body), "body is exactly counts + FString pairs");

        let mut off = 0;
        for _ in 0..2 {
            let n = i32::from_le_bytes(v[off..off + 4].try_into().unwrap());
            off += 4 + n as usize;
        }
        assert_eq!(i32::from_le_bytes(v[off..off + 4].try_into().unwrap()) as usize, body.len());
        let types = &v[off + 8..off + 8 + 2 * fstring_vec("StrProperty").len()];
        assert_eq!(types, [fstring_vec("StrProperty"), fstring_vec("StrProperty")].concat());
    }

    #[test]
    fn map_body_carries_num_keys_to_remove() {
        let mut v = Vec::new();
        prop_map_i32_i32(&mut v, "SelfByChara", &[(37, 2248), (33, 2108)]);
        assert!(v.windows(4).any(|w| w == 24i32.to_le_bytes()));
        let tail = &v[v.len() - 24..];
        assert_eq!(&tail[0..4], &0i32.to_le_bytes(), "NumKeysToRemove");
        assert_eq!(&tail[4..8], &2i32.to_le_bytes(), "NumEntries");
    }
}
