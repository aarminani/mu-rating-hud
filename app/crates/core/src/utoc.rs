use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum ScanError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("not an IoStore container")]
    NotAContainer,
    #[error("container is truncated: needed {need} bytes, file has {have}")]
    Truncated { need: usize, have: usize },
    #[error("container directory index is encrypted")]
    Encrypted,
    #[error("unsupported container version {0}")]
    UnsupportedVersion(u8),
    #[error("malformed container: {0}")]
    Malformed(&'static str),
}

pub type ScanResult<T> = Result<T, ScanError>;

const TOC_MAGIC: &[u8; 16] = b"-==--==--==--==-";
const HEADER_SIZE: usize = 144;
const INVALID_INDEX: u32 = u32::MAX;

mod flags {
    pub const COMPRESSED: u8 = 0x01;
    pub const ENCRYPTED: u8 = 0x02;
    pub const SIGNED: u8 = 0x04;
    pub const INDEXED: u8 = 0x08;
}

const V_DIRECTORY_INDEX: u8 = 2;
const V_PARTITION_SIZE: u8 = 3;
const V_PERFECT_HASH: u8 = 4;
const V_PERFECT_HASH_OVERFLOW: u8 = 5;
const V_MAX_KNOWN: u8 = 8;

#[derive(Debug, Clone)]
pub struct TocInfo {
    pub version: u8,
    pub container_id: u64,
    pub chunk_count: u32,
    pub compressed: bool,
    pub encrypted: bool,
    pub signed: bool,
    pub indexed: bool,
    pub mount_point: String,
    pub asset_paths: Vec<String>,
}

struct Cursor<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn new(buf: &'a [u8]) -> Self {
        Cursor { buf, pos: 0 }
    }

    fn seek(&mut self, pos: usize) -> ScanResult<()> {
        if pos > self.buf.len() {
            return Err(ScanError::Truncated {
                need: pos,
                have: self.buf.len(),
            });
        }
        self.pos = pos;
        Ok(())
    }

    fn take(&mut self, n: usize) -> ScanResult<&'a [u8]> {
        let end = self.pos.checked_add(n).ok_or(ScanError::Malformed(
            "length overflow while reading container",
        ))?;
        if end > self.buf.len() {
            return Err(ScanError::Truncated {
                need: end,
                have: self.buf.len(),
            });
        }
        let out = &self.buf[self.pos..end];
        self.pos = end;
        Ok(out)
    }

    fn u32(&mut self) -> ScanResult<u32> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn i32(&mut self) -> ScanResult<i32> {
        Ok(self.u32()? as i32)
    }

    fn u64(&mut self) -> ScanResult<u64> {
        let b = self.take(8)?;
        Ok(u64::from_le_bytes([
            b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
        ]))
    }

    fn fstring(&mut self) -> ScanResult<String> {
        let len = self.i32()?;
        if len == 0 {
            return Ok(String::new());
        }
        if len > 0 {
            let n = len as usize;
            if n > self.buf.len() {
                return Err(ScanError::Malformed("string length exceeds container"));
            }
            let raw = self.take(n)?;
            let trimmed = raw.strip_suffix(&[0]).unwrap_or(raw);
            Ok(String::from_utf8_lossy(trimmed).into_owned())
        } else {
            let n = len
                .checked_neg()
                .ok_or(ScanError::Malformed("string length overflow"))? as usize;
            let raw = self.take(n.checked_mul(2).ok_or(ScanError::Malformed(
                "utf16 string length overflow",
            ))?)?;
            let units: Vec<u16> = raw
                .chunks_exact(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .take_while(|&u| u != 0)
                .collect();
            Ok(String::from_utf16_lossy(&units))
        }
    }
}

#[derive(Clone, Copy)]
struct DirEntry {
    name: u32,
    first_child: u32,
    next_sibling: u32,
    first_file: u32,
}

#[derive(Clone, Copy)]
struct FileEntry {
    name: u32,
    next_file: u32,
}

pub fn parse_file(path: &Path) -> ScanResult<TocInfo> {
    let bytes = std::fs::read(path)?;
    parse_bytes(&bytes)
}

pub fn parse_bytes(bytes: &[u8]) -> ScanResult<TocInfo> {
    if bytes.len() < HEADER_SIZE {
        return Err(ScanError::Truncated {
            need: HEADER_SIZE,
            have: bytes.len(),
        });
    }
    if &bytes[0..16] != TOC_MAGIC {
        return Err(ScanError::NotAContainer);
    }

    let mut c = Cursor::new(bytes);
    c.seek(16)?;

    let version = c.take(1)?[0];
    c.take(3)?;

    let header_size = c.u32()? as usize;
    let chunk_count = c.u32()?;
    let compressed_block_count = c.u32()?;
    let compressed_block_size = c.u32()?;
    let method_name_count = c.u32()?;
    let method_name_length = c.u32()?;
    let _compression_block_size = c.u32()?;
    let directory_index_size = c.u32()? as usize;
    let _partition_count = c.u32()?;
    let container_id = c.u64()?;
    c.take(16)?;
    let container_flags = c.take(1)?[0];
    c.take(3)?;
    let perfect_hash_seed_count = c.u32()?;
    let _partition_size = c.u64()?;
    let chunks_without_perfect_hash = c.u32()?;

    if version < V_DIRECTORY_INDEX {
        return Err(ScanError::UnsupportedVersion(version));
    }
    if version > V_MAX_KNOWN {
        return Err(ScanError::UnsupportedVersion(version));
    }
    if header_size != HEADER_SIZE {
        return Err(ScanError::Malformed("unexpected TOC header size"));
    }
    if compressed_block_size != 12 {
        return Err(ScanError::Malformed("unexpected compressed block entry size"));
    }

    let encrypted = container_flags & flags::ENCRYPTED != 0;
    let indexed = container_flags & flags::INDEXED != 0;
    let is_signed = container_flags & flags::SIGNED != 0;

    let mut info = TocInfo {
        version,
        container_id,
        chunk_count,
        compressed: container_flags & flags::COMPRESSED != 0,
        encrypted,
        signed: is_signed,
        indexed,
        mount_point: String::new(),
        asset_paths: Vec::new(),
    };

    if encrypted {
        return Err(ScanError::Encrypted);
    }
    if !indexed || directory_index_size == 0 {
        return Ok(info);
    }

    let mut off = HEADER_SIZE;
    off += chunk_count as usize * 12;
    off += chunk_count as usize * 10;
    if version >= V_PERFECT_HASH {
        off += perfect_hash_seed_count as usize * 4;
    }
    if version >= V_PERFECT_HASH_OVERFLOW {
        off += chunks_without_perfect_hash as usize * 4;
    }
    off += compressed_block_count as usize * 12;
    off += (method_name_count as usize) * (method_name_length as usize);
    if is_signed {
        let mut sc = Cursor::new(bytes);
        sc.seek(off)?;
        let hash_size = sc.u32()? as usize;
        off += 4 + hash_size * 2 + compressed_block_count as usize * hash_size;
    }
    let _ = V_PARTITION_SIZE;

    let index_end = off
        .checked_add(directory_index_size)
        .ok_or(ScanError::Malformed("directory index offset overflow"))?;
    if index_end > bytes.len() {
        return Err(ScanError::Truncated {
            need: index_end,
            have: bytes.len(),
        });
    }

    let index = &bytes[off..index_end];
    let (mount_point, paths) = parse_directory_index(index)?;
    info.mount_point = mount_point;
    info.asset_paths = paths;
    Ok(info)
}

fn parse_directory_index(buf: &[u8]) -> ScanResult<(String, Vec<String>)> {
    let mut c = Cursor::new(buf);

    let mount_point = c.fstring()?;

    let dir_count = c.i32()?;
    if dir_count < 0 {
        return Err(ScanError::Malformed("negative directory entry count"));
    }
    let mut dirs = Vec::with_capacity(dir_count.min(1 << 20) as usize);
    for _ in 0..dir_count {
        dirs.push(DirEntry {
            name: c.u32()?,
            first_child: c.u32()?,
            next_sibling: c.u32()?,
            first_file: c.u32()?,
        });
    }

    let file_count = c.i32()?;
    if file_count < 0 {
        return Err(ScanError::Malformed("negative file entry count"));
    }
    let mut files = Vec::with_capacity(file_count.min(1 << 20) as usize);
    for _ in 0..file_count {
        let name = c.u32()?;
        let next_file = c.u32()?;
        let _user_data = c.u32()?;
        files.push(FileEntry { name, next_file });
    }

    let string_count = c.i32()?;
    if string_count < 0 {
        return Err(ScanError::Malformed("negative string table count"));
    }
    let mut strings = Vec::with_capacity(string_count.min(1 << 20) as usize);
    for _ in 0..string_count {
        strings.push(c.fstring()?);
    }

    let name_of = |idx: u32| -> &str {
        if idx == INVALID_INDEX {
            ""
        } else {
            strings.get(idx as usize).map(|s| s.as_str()).unwrap_or("")
        }
    };

    let mut out = Vec::new();
    if dirs.is_empty() {
        return Ok((mount_point, out));
    }

    let prefix = normalize_mount(&mount_point);

    let mut stack: Vec<(u32, String)> = vec![(0, String::new())];
    let mut visits = 0usize;
    let budget = dirs.len().saturating_mul(4) + 16;

    while let Some((dir_idx, base)) = stack.pop() {
        if dir_idx == INVALID_INDEX {
            continue;
        }
        visits += 1;
        if visits > budget {
            return Err(ScanError::Malformed("cycle in directory index"));
        }
        let Some(dir) = dirs.get(dir_idx as usize).copied() else {
            continue;
        };

        let here = if dir.name == INVALID_INDEX {
            base.clone()
        } else {
            let n = name_of(dir.name);
            if base.is_empty() {
                n.to_string()
            } else {
                format!("{base}/{n}")
            }
        };

        let mut file_idx = dir.first_file;
        let mut file_visits = 0usize;
        while file_idx != INVALID_INDEX {
            file_visits += 1;
            if file_visits > files.len() + 1 {
                return Err(ScanError::Malformed("cycle in file index"));
            }
            let Some(f) = files.get(file_idx as usize).copied() else {
                break;
            };
            let name = name_of(f.name);
            let full = if here.is_empty() {
                format!("{prefix}{name}")
            } else {
                format!("{prefix}{here}/{name}")
            };
            out.push(full);
            file_idx = f.next_file;
        }

        if dir.next_sibling != INVALID_INDEX {
            stack.push((dir.next_sibling, base));
        }
        if dir.first_child != INVALID_INDEX {
            stack.push((dir.first_child, here));
        }
    }

    out.sort_unstable();
    out.dedup();
    Ok((mount_point, out))
}

fn normalize_mount(mount: &str) -> String {
    let cleaned = mount.replace('\\', "/");
    let stripped = cleaned.trim_start_matches("../");
    let stripped = stripped.trim_start_matches('/');
    if stripped.is_empty() {
        String::new()
    } else if stripped.ends_with('/') {
        stripped.to_string()
    } else {
        format!("{stripped}/")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(version: u8, flags: u8, dir_index_size: u32, chunk_count: u32) -> Vec<u8> {
        let mut h = Vec::with_capacity(HEADER_SIZE);
        h.extend_from_slice(TOC_MAGIC);
        h.push(version);
        h.extend_from_slice(&[0, 0, 0]);
        h.extend_from_slice(&(HEADER_SIZE as u32).to_le_bytes());
        h.extend_from_slice(&chunk_count.to_le_bytes());
        h.extend_from_slice(&0u32.to_le_bytes());
        h.extend_from_slice(&12u32.to_le_bytes());
        h.extend_from_slice(&0u32.to_le_bytes());
        h.extend_from_slice(&32u32.to_le_bytes());
        h.extend_from_slice(&65536u32.to_le_bytes());
        h.extend_from_slice(&dir_index_size.to_le_bytes());
        h.extend_from_slice(&1u32.to_le_bytes());
        h.extend_from_slice(&0xDEADBEEFu64.to_le_bytes());
        h.extend_from_slice(&[0u8; 16]);
        h.push(flags);
        h.extend_from_slice(&[0, 0, 0]);
        h.extend_from_slice(&0u32.to_le_bytes());
        h.extend_from_slice(&u64::MAX.to_le_bytes());
        h.extend_from_slice(&0u32.to_le_bytes());
        h.extend_from_slice(&0u32.to_le_bytes());
        h.extend_from_slice(&[0u8; 40]);
        assert_eq!(h.len(), HEADER_SIZE);
        h
    }

    fn fstring(s: &str) -> Vec<u8> {
        let mut out = Vec::new();
        let bytes = s.as_bytes();
        out.extend_from_slice(&((bytes.len() + 1) as i32).to_le_bytes());
        out.extend_from_slice(bytes);
        out.push(0);
        out
    }

    fn sample_index() -> Vec<u8> {
        let mut idx = Vec::new();
        idx.extend_from_slice(&fstring("../../../Polaris/Content/"));

        idx.extend_from_slice(&2i32.to_le_bytes());
        idx.extend_from_slice(&INVALID_INDEX.to_le_bytes());
        idx.extend_from_slice(&1u32.to_le_bytes());
        idx.extend_from_slice(&INVALID_INDEX.to_le_bytes());
        idx.extend_from_slice(&0u32.to_le_bytes());
        idx.extend_from_slice(&0u32.to_le_bytes());
        idx.extend_from_slice(&INVALID_INDEX.to_le_bytes());
        idx.extend_from_slice(&INVALID_INDEX.to_le_bytes());
        idx.extend_from_slice(&1u32.to_le_bytes());

        idx.extend_from_slice(&2i32.to_le_bytes());
        idx.extend_from_slice(&1u32.to_le_bytes());
        idx.extend_from_slice(&INVALID_INDEX.to_le_bytes());
        idx.extend_from_slice(&0u32.to_le_bytes());
        idx.extend_from_slice(&2u32.to_le_bytes());
        idx.extend_from_slice(&INVALID_INDEX.to_le_bytes());
        idx.extend_from_slice(&0u32.to_le_bytes());

        idx.extend_from_slice(&3i32.to_le_bytes());
        idx.extend_from_slice(&fstring("Chara"));
        idx.extend_from_slice(&fstring("root.uasset"));
        idx.extend_from_slice(&fstring("zaf.uasset"));
        idx
    }

    #[test]
    fn rejects_non_container() {
        let err = parse_bytes(&[0u8; 200]).unwrap_err();
        assert!(matches!(err, ScanError::NotAContainer));
    }

    #[test]
    fn rejects_truncated() {
        let err = parse_bytes(&header(5, 0x09, 0, 0)[..100]).unwrap_err();
        assert!(matches!(err, ScanError::Truncated { .. }));
    }

    #[test]
    fn refuses_encrypted_rather_than_guessing() {
        let buf = header(5, flags::ENCRYPTED | flags::INDEXED, 64, 0);
        assert!(matches!(parse_bytes(&buf).unwrap_err(), ScanError::Encrypted));
    }

    #[test]
    fn unindexed_container_is_valid_but_pathless() {
        let buf = header(5, flags::COMPRESSED, 0, 0);
        let info = parse_bytes(&buf).unwrap();
        assert!(info.asset_paths.is_empty());
        assert!(!info.indexed);
        assert_eq!(info.container_id, 0xDEADBEEF);
    }

    #[test]
    fn extracts_paths_with_mount_point_applied() {
        let idx = sample_index();
        let mut buf = header(5, flags::COMPRESSED | flags::INDEXED, idx.len() as u32, 0);
        buf.extend_from_slice(&idx);

        let info = parse_bytes(&buf).unwrap();
        assert_eq!(info.version, 5);
        assert_eq!(
            info.asset_paths,
            vec![
                "Polaris/Content/Chara/zaf.uasset".to_string(),
                "Polaris/Content/root.uasset".to_string(),
            ]
        );
    }

    #[test]
    fn detects_cycles_in_directory_links() {
        let mut idx = Vec::new();
        idx.extend_from_slice(&fstring("/"));
        idx.extend_from_slice(&1i32.to_le_bytes());
        idx.extend_from_slice(&INVALID_INDEX.to_le_bytes());
        idx.extend_from_slice(&0u32.to_le_bytes());
        idx.extend_from_slice(&INVALID_INDEX.to_le_bytes());
        idx.extend_from_slice(&INVALID_INDEX.to_le_bytes());
        idx.extend_from_slice(&0i32.to_le_bytes());
        idx.extend_from_slice(&0i32.to_le_bytes());

        let mut buf = header(5, flags::INDEXED, idx.len() as u32, 0);
        buf.extend_from_slice(&idx);
        assert!(matches!(
            parse_bytes(&buf).unwrap_err(),
            ScanError::Malformed(_)
        ));
    }

    #[test]
    fn mount_normalization_strips_relative_hops() {
        assert_eq!(normalize_mount("../../../"), "");
        assert_eq!(normalize_mount("../../../Polaris/Content/"), "Polaris/Content/");
        assert_eq!(normalize_mount("Polaris/Content"), "Polaris/Content/");
        assert_eq!(normalize_mount("/"), "");
    }
}
