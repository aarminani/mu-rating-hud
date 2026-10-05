use std::io::Read;
use std::path::Path;
use std::time::Duration;

use serde::Deserialize;

pub const REPO: &str = "aarminani/mu-rating-hud";

const USER_AGENT: &str = concat!("MuRatingHelper/", env!("CARGO_PKG_VERSION"), " (+https://tekkenresourcehub.com)");

const TIMEOUT: Duration = Duration::from_secs(8);

#[derive(Debug, thiserror::Error)]
pub enum UpdateError {
    #[error("github: {0}")]
    Http(String),
    #[error("github sent something this build cannot read: {0}")]
    Shape(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    pub tag: String,
    pub page: String,
    pub asset: Option<Asset>,
    pub exe: Option<Asset>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Asset {
    pub name: String,
    pub url: String,
    pub size: u64,
}

#[derive(Deserialize)]
struct ApiRelease {
    tag_name: String,
    #[serde(default)]
    html_url: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    assets: Vec<ApiAsset>,
}

#[derive(Deserialize)]
struct ApiAsset {
    name: String,
    browser_download_url: String,
    #[serde(default)]
    size: u64,
}

fn parts(v: &str) -> Option<(u32, u32, u32)> {
    let v = v.trim().trim_start_matches(['v', 'V']);
    let v = v.split(['-', '+']).next()?;
    let mut it = v.split('.');
    let a = it.next()?.parse().ok()?;
    let b = it.next().unwrap_or("0").parse().ok()?;
    let c = it.next().unwrap_or("0").parse().ok()?;
    if it.next().is_some() {
        return None;
    }
    Some((a, b, c))
}

pub fn is_newer(running: &str, candidate: &str) -> bool {
    match (parts(running), parts(candidate)) {
        (Some(r), Some(c)) => c > r,
        _ => false,
    }
}

pub fn running_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

fn helper_asset(assets: &[ApiAsset]) -> Option<Asset> {
    assets
        .iter()
        .find(|a| {
            let n = a.name.to_ascii_lowercase();
            n.starts_with("trhmu") && n.ends_with(".zip")
        })
        .map(pick)
}

fn exe_asset(assets: &[ApiAsset]) -> Option<Asset> {
    assets.iter().find(|a| a.name.eq_ignore_ascii_case(EXE_NAME)).map(pick)
}

fn pick(a: &ApiAsset) -> Asset {
    Asset { name: a.name.clone(), url: a.browser_download_url.clone(), size: a.size }
}

pub const EXE_NAME: &str = "trhmu.exe";

pub fn parse_latest(body: &str) -> Result<Option<Release>, UpdateError> {
    let r: ApiRelease = serde_json::from_str(body).map_err(|e| UpdateError::Shape(e.to_string()))?;
    if r.draft || r.prerelease {
        return Ok(None);
    }
    Ok(Some(Release {
        tag: r.tag_name,
        page: r.html_url,
        asset: helper_asset(&r.assets),
        exe: exe_asset(&r.assets),
    }))
}

pub fn latest() -> Result<Option<Release>, UpdateError> {
    let url = format!("https://api.github.com/repos/{REPO}/releases/latest");
    let resp = ureq::builder()
        .timeout(TIMEOUT)
        .build()
        .get(&url)
        .set("User-Agent", USER_AGENT)
        .set("Accept", "application/vnd.github+json")
        .call();
    match resp {
        Ok(r) => {
            let body = r.into_string().map_err(|e| UpdateError::Http(e.to_string()))?;
            parse_latest(&body)
        }
        Err(ureq::Error::Status(404, _)) => Ok(None),
        Err(e) => Err(UpdateError::Http(e.to_string())),
    }
}

pub fn download_exe(url: &str, dest: &Path, expected: u64) -> Result<(), UpdateError> {
    const SANE_MAX: u64 = 64 * 1024 * 1024;
    if expected == 0 || expected > SANE_MAX {
        return Err(UpdateError::Shape(format!("refusing a {expected} byte download")));
    }
    let resp = ureq::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .get(url)
        .set("User-Agent", USER_AGENT)
        .call()
        .map_err(|e| UpdateError::Http(e.to_string()))?;

    let mut bytes = Vec::with_capacity(expected as usize);
    resp.into_reader()
        .take(SANE_MAX + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| UpdateError::Http(e.to_string()))?;

    check_exe(&bytes, expected)?;
    std::fs::write(dest, &bytes).map_err(|e| UpdateError::Http(e.to_string()))?;
    Ok(())
}

pub fn check_exe(bytes: &[u8], expected: u64) -> Result<(), UpdateError> {
    if bytes.len() as u64 != expected {
        return Err(UpdateError::Shape(format!(
            "expected {expected} bytes, got {}",
            bytes.len()
        )));
    }
    if bytes.first_chunk::<2>() != Some(b"MZ") {
        return Err(UpdateError::Shape(
            "not a Windows executable (no MZ header); a proxy or an error page, not the helper".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newer_is_numeric_not_lexical() {
        assert!(is_newer("0.1.9", "0.1.10"), "a string compare gets this one backwards");
        assert!(is_newer("0.1.1", "0.2.0"));
        assert!(is_newer("0.9.9", "1.0.0"));
        assert!(is_newer("0.1.1", "v0.1.2"), "tags carry a leading v");
    }

    #[test]
    fn the_running_build_is_never_offered_to_itself() {
        assert!(!is_newer("0.1.1", "0.1.1"));
        assert!(!is_newer("0.1.1", "v0.1.1"));
    }

    #[test]
    fn a_downgrade_is_never_offered() {
        assert!(!is_newer("0.1.2", "0.1.1"));
        assert!(!is_newer("1.0.0", "0.9.9"));
    }

    #[test]
    fn a_version_we_cannot_read_moves_nobody() {
        for junk in ["", "latest", "0.1.1.1", "v", "nightly-2026-09-25", "0.x.1"] {
            assert!(!is_newer("0.1.1", junk), "{junk:?} must not be offered");
            assert!(!is_newer(junk, "9.9.9"), "and must not make everything newer either");
        }
        assert_eq!(parts("0.1"), Some((0, 1, 0)));
        assert_eq!(parts("0.1.1.1"), None, "four parts is not a version this app publishes");
    }

    #[test]
    fn a_prerelease_suffix_compares_on_its_numbers() {
        assert!(is_newer("0.1.1", "0.1.2-beta"));
        assert!(!is_newer("0.1.2", "0.1.2-beta"));
    }

    const BODY: &str = r#"{
        "tag_name": "v0.1.1",
        "html_url": "https://github.com/aarminani/mu-rating-hud/releases/tag/v0.1.1",
        "draft": false,
        "prerelease": false,
        "assets": [
            {"name": "TrueProwessMuRating-0.1.0.zip",
             "browser_download_url": "https://example.invalid/mod.zip", "size": 64905},
            {"name": "trhmu-0.1.1.zip",
             "browser_download_url": "https://example.invalid/trhmu.zip", "size": 5517041}
        ]
    }"#;

    #[test]
    fn the_helper_zip_is_picked_and_not_the_mod_zip() {
        let r = parse_latest(BODY).unwrap().expect("a published release");
        assert_eq!(r.tag, "v0.1.1");
        let a = r.asset.expect("the helper zip");
        assert_eq!(a.name, "trhmu-0.1.1.zip");
        assert_eq!(a.size, 5_517_041);
    }

    #[test]
    fn a_release_with_no_helper_zip_still_parses_and_offers_the_page() {
        let body = BODY.replace("trhmu-0.1.1.zip", "notes.txt");
        let r = parse_latest(&body).unwrap().expect("a published release");
        assert!(r.asset.is_none(), "nothing to install, but the page is still worth opening");
        assert!(r.page.ends_with("v0.1.1"));
    }

    #[test]
    fn drafts_and_prereleases_are_not_offered() {
        assert!(parse_latest(&BODY.replace("\"draft\": false", "\"draft\": true")).unwrap().is_none());
        assert!(parse_latest(&BODY.replace("\"prerelease\": false", "\"prerelease\": true")).unwrap().is_none());
    }

    #[test]
    fn a_body_that_is_not_a_release_is_an_error_not_a_panic() {
        assert!(parse_latest("{}").is_err(), "no tag_name");
        assert!(parse_latest("not json").is_err());
    }

    #[test]
    fn this_build_knows_its_own_version() {
        assert!(parts(running_version()).is_some(), "Cargo's version must parse");
    }

    const WITH_EXE: &str = r#"{
        "tag_name": "v1.0.2",
        "html_url": "https://github.com/aarminani/mu-rating-hud/releases/tag/v1.0.2",
        "draft": false,
        "prerelease": false,
        "assets": [
            {"name": "trhmu-1.0.2.zip", "browser_download_url": "https://e.invalid/z", "size": 5},
            {"name": "trhmu.exe", "browser_download_url": "https://e.invalid/exe", "size": 8456704},
            {"name": "trhmu-dash.exe", "browser_download_url": "https://e.invalid/dash", "size": 9},
            {"name": "trhmu-preview.exe", "browser_download_url": "https://e.invalid/prev", "size": 7}
        ]
    }"#;

    #[test]
    fn the_installable_exe_is_matched_by_exact_name() {
        let r = parse_latest(WITH_EXE).unwrap().unwrap();
        let exe = r.exe.expect("the loose exe");
        assert_eq!(exe.name, "trhmu.exe", "never the dash or preview build");
        assert_eq!(exe.size, 8_456_704);
        assert_eq!(r.asset.unwrap().name, "trhmu-1.0.2.zip", "the zip is still found too");
    }

    #[test]
    fn a_release_without_the_loose_exe_cannot_be_installed_in_place() {
        let r = parse_latest(BODY).unwrap().unwrap();
        assert!(r.exe.is_none());
        assert!(r.asset.is_some());
    }

    #[test]
    fn a_short_download_is_refused() {
        let mut exe = b"MZ".to_vec();
        exe.extend_from_slice(&[0u8; 98]);
        assert!(check_exe(&exe, 100).is_ok(), "exact length and an MZ header");
        assert!(check_exe(&exe, 101).is_err(), "one byte short is a truncated download");
        assert!(check_exe(&exe, 99).is_err());
    }

    #[test]
    fn an_error_page_is_not_installed_over_the_helper() {
        let html = b"<!DOCTYPE html><html><body>Sign in to continue</body></html>".to_vec();
        let err = check_exe(&html, html.len() as u64).unwrap_err();
        assert!(err.to_string().contains("MZ"), "refused for the right reason: {err}");
    }

    #[test]
    fn the_real_shipped_exe_passes_both_gates() {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../release/trhmu-1.0.1/trhmu.exe");
        if let Ok(bytes) = std::fs::read(&p) {
            assert!(check_exe(&bytes, bytes.len() as u64).is_ok(), "the shipped exe must pass");
        }
    }
}
