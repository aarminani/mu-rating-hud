use std::path::{Path, PathBuf};

pub const APP_ID: &str = "1778820";

fn vdf_values(text: &str, key: &str) -> Vec<String> {
    let needle = format!("\"{key}\"");
    let mut out = Vec::new();
    let mut from = 0usize;
    while let Some(i) = text[from..].find(&needle).map(|p| from + p) {
        let after = i + needle.len();
        let rest = &text[after..];
        if let Some(a) = rest.find('"') {
            if let Some(b) = rest[a + 1..].find('"') {
                out.push(rest[a + 1..a + 1 + b].to_string());
            }
        }
        from = after;
    }
    out
}

pub(crate) fn steam_root() -> Option<PathBuf> {
    #[cfg(windows)]
    if let Some(p) = registry_steam_path() {
        if p.is_dir() {
            return Some(p);
        }
    }
    for var in ["ProgramFiles(x86)", "ProgramFiles"] {
        if let Ok(pf) = std::env::var(var) {
            let p = Path::new(&pf).join("Steam");
            if p.is_dir() {
                return Some(p);
            }
        }
    }
    None
}

#[cfg(windows)]
fn registry_steam_path() -> Option<PathBuf> {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;
    let val: String = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(r"Software\Valve\Steam")
        .ok()?
        .get_value("SteamPath")
        .ok()?;
    let val = val.trim();
    if val.is_empty() {
        None
    } else {
        Some(PathBuf::from(val.replace('/', "\\")))
    }
}

fn libraries(root: &Path) -> Vec<PathBuf> {
    let mut libs = vec![root.to_path_buf()];
    let vdf = root.join("steamapps").join("libraryfolders.vdf");
    if let Ok(text) = std::fs::read_to_string(&vdf) {
        for p in vdf_values(&text, "path") {
            let p = PathBuf::from(p.replace("\\\\", "\\"));
            if !libs.contains(&p) {
                libs.push(p);
            }
        }
    }
    libs
}

fn manifest() -> Option<(PathBuf, String)> {
    let root = steam_root()?;
    libraries(&root).into_iter().find_map(|lib| {
        let path = lib
            .join("steamapps")
            .join(format!("appmanifest_{APP_ID}.acf"));
        std::fs::read_to_string(path).ok().map(|text| (lib, text))
    })
}

fn number(text: &str, key: &str) -> Option<u64> {
    vdf_values(text, key).into_iter().next()?.parse().ok()
}

pub fn build_id() -> Option<u64> {
    manifest().and_then(|(_, text)| number(&text, "buildid"))
}

pub fn pending_build() -> Option<u64> {
    manifest().and_then(|(_, text)| pending_build_in(&text))
}

fn pending_build_in(text: &str) -> Option<u64> {
    let installed = number(text, "buildid")?;
    let target = number(text, "TargetBuildID")?;
    (target > installed).then_some(target)
}

pub fn last_updated() -> Option<i64> {
    manifest().and_then(|(_, text)| number(&text, "LastUpdated")).map(|t| t as i64)
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct DisplayMode {
    pub mode: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

pub fn parse_display_mode(ini: &str) -> Option<DisplayMode> {
    let value = |key: &str| {
        ini.lines()
            .map(str::trim)
            .find_map(|l| l.strip_prefix(key).and_then(|rest| rest.strip_prefix('=')))
            .map(str::trim)
    };
    let mode = match value("FullscreenMode")? {
        "0" => "fullscreen",
        "1" => "borderless",
        "2" => "windowed",
        other => return Some(DisplayMode { mode: format!("unknown ({other})"), width: None, height: None }),
    };
    Some(DisplayMode {
        mode: mode.to_string(),
        width: value("ResolutionSizeX").and_then(|v| v.parse().ok()),
        height: value("ResolutionSizeY").and_then(|v| v.parse().ok()),
    })
}

pub fn display_mode() -> Option<DisplayMode> {
    let ini = crate::paths::saves_root().parent()?.join("Config").join("Windows").join("GameUserSettings.ini");
    parse_display_mode(&std::fs::read_to_string(ini).ok()?)
}

pub fn install_dir() -> Option<PathBuf> {
    let (lib, text) = manifest()?;
    let dir = vdf_values(&text, "installdir").into_iter().next()?;
    Some(lib.join("steamapps").join("common").join(dir))
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct ModFile {
    pub path: String,
    pub bytes: u64,
}

pub fn paks_dir() -> Option<PathBuf> {
    install_dir().map(|d| d.join("Polaris").join("Content").join("Paks"))
}

pub fn mod_files() -> Vec<ModFile> {
    let Some(root) = paks_dir() else { return Vec::new() };
    let mut out = Vec::new();
    find_mod_files(&root, &root, 0, &mut out);
    out.sort_by(|a, b| a.path.cmp(&b.path));
    out
}

fn find_mod_files(root: &Path, dir: &Path, depth: usize, out: &mut Vec<ModFile>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(kind) = entry.file_type() else { continue };
        if kind.is_dir() {
            if depth < 4 {
                find_mod_files(root, &path, depth + 1, out);
            }
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
        let wanted = name.contains("murating")
            && (name.ends_with(".pak") || name.ends_with(".utoc") || name.ends_with(".ucas"));
        if wanted {
            let rel = path.strip_prefix(root).unwrap_or(&path).to_string_lossy().replace('\\', "/");
            out.push(ModFile { path: rel, bytes: entry.metadata().map(|m| m.len()).unwrap_or(0) });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
"AppState"
{
	"appid"		"1778820"
	"name"		"TEKKEN 8"
	"StateFlags"		"4"
	"installdir"		"TEKKEN 8"
	"buildid"		"25221600"
}
"#;

    #[test]
    fn pulls_the_build_id_out_of_a_manifest() {
        assert_eq!(vdf_values(SAMPLE, "buildid"), vec!["25221600".to_string()]);
        assert_eq!(vdf_values(SAMPLE, "name"), vec!["TEKKEN 8".to_string()]);
    }

    #[test]
    fn a_pending_patch_is_a_target_ahead_of_the_install() {
        assert_eq!(pending_build_in(SAMPLE), None, "no TargetBuildID at all");
        let settled = SAMPLE.replace("}", "\t\"TargetBuildID\"\t\t\"25221600\"\n}");
        assert_eq!(pending_build_in(&settled), None, "equal means nothing pending");
        let queued = SAMPLE.replace("}", "\t\"TargetBuildID\"\t\t\"25310000\"\n}");
        assert_eq!(pending_build_in(&queued), Some(25310000));
    }

    #[test]
    fn reads_tekkens_window_mode() {
        let ini = "[/Script/Engine.GameUserSettings]\nResolutionSizeX=1920\nResolutionSizeY=1080\n\
                   LastUserConfirmedResolutionSizeX=1600\nFullscreenMode=1\nLastConfirmedFullscreenMode=2\n\
                   PreferredFullscreenMode=1\n";
        assert_eq!(
            parse_display_mode(ini),
            Some(DisplayMode { mode: "borderless".into(), width: Some(1920), height: Some(1080) })
        );
        assert_eq!(parse_display_mode("FullscreenMode=0").map(|d| d.mode), Some("fullscreen".into()));
        assert_eq!(parse_display_mode("LastConfirmedFullscreenMode=2"), None, "only the key itself");
    }

    #[test]
    fn missing_key_is_empty_not_a_panic() {
        assert!(vdf_values(SAMPLE, "nosuchkey").is_empty());
        assert!(vdf_values("", "buildid").is_empty());
    }

    #[test]
    fn reads_every_library_path() {
        let lf = r#"
"libraryfolders"
{
	"0"
	{
		"path"		"C:\\Program Files (x86)\\Steam"
	}
	"1"
	{
		"path"		"D:\\SteamLibrary"
	}
}
"#;
        let paths = vdf_values(lf, "path");
        assert_eq!(paths.len(), 2);
        assert!(paths[1].contains("SteamLibrary"));
    }
}
