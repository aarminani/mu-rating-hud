use std::collections::{BTreeSet, VecDeque};
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Mutex, OnceLock};

use serde::Serialize;
use tauri::{AppHandle, Manager};

use murating_core::game::{self, DisplayMode, ModFile};
use murating_core::{paths, report, slot};

use crate::{feed, follow, tray};

const KEEP: usize = 120;
const LOG_LINES: usize = 40;
const ERROR_LINES: usize = 10;

static LINES: Mutex<VecDeque<(i64, String)>> = Mutex::new(VecDeque::new());
static DATA_DIR: OnceLock<std::path::PathBuf> = OnceLock::new();
static STARTED: AtomicI64 = AtomicI64::new(0);

#[macro_export]
macro_rules! diag {
    ($($t:tt)*) => {{
        let line = format!($($t)*);
        eprintln!("{line}");
        $crate::diag::push(line);
    }};
}

pub fn push(line: String) {
    if let Ok(mut lines) = LINES.lock() {
        if lines.len() >= KEEP {
            lines.pop_front();
        }
        lines.push_back((feed::now(), line));
    }
}

fn log_lines() -> Vec<LogLine> {
    LINES
        .lock()
        .map(|l| l.iter().map(|(at, text)| LogLine { at: *at, text: text.clone() }).collect())
        .unwrap_or_default()
}

fn is_error(text: &str) -> bool {
    let t = text.to_ascii_lowercase();
    ["error", "failed", "refused", "could not", "panic", "missing", "timed out", "not found"]
        .iter()
        .any(|w| t.contains(w))
}

#[derive(Clone, Copy)]
pub enum Endpoint {
    Feed,
    Wavu,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct NetStat {
    pub last_ok_at: Option<i64>,
    pub last_ms: Option<u64>,
    pub last_error: Option<String>,
    pub last_error_at: Option<i64>,
    pub requests: u64,
    pub errors: u64,
}

impl NetStat {
    const EMPTY: NetStat =
        NetStat { last_ok_at: None, last_ms: None, last_error: None, last_error_at: None, requests: 0, errors: 0 };
}

static NET: Mutex<[NetStat; 2]> = Mutex::new([NetStat::EMPTY, NetStat::EMPTY]);
static SKEW: Mutex<Option<i64>> = Mutex::new(None);

pub fn net_ok(e: Endpoint, ms: u64, server_date: Option<i64>) {
    let now = feed::now();
    if let Ok(mut net) = NET.lock() {
        let s = &mut net[e as usize];
        s.requests += 1;
        s.last_ok_at = Some(now);
        s.last_ms = Some(ms);
    }
    if let (Some(date), Ok(mut skew)) = (server_date, SKEW.lock()) {
        *skew = Some(now - date);
    }
}

pub fn net_err(e: Endpoint, error: impl std::fmt::Display) {
    if let Ok(mut net) = NET.lock() {
        let s = &mut net[e as usize];
        s.requests += 1;
        s.errors += 1;
        s.last_error = Some(error.to_string());
        s.last_error_at = Some(feed::now());
    }
}

pub fn timed<T>(f: impl FnOnce() -> T) -> (T, u64) {
    let start = std::time::Instant::now();
    let out = f();
    (out, start.elapsed().as_millis() as u64)
}

fn net(e: Endpoint) -> NetStat {
    NET.lock().map(|n| n[e as usize].clone()).unwrap_or(NetStat::EMPTY)
}

pub(crate) fn civil(t: i64) -> (i64, i64, i64, i64) {
    let days = t.div_euclid(86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (yoe + era * 400 + i64::from(m <= 2), m, d, t.rem_euclid(86_400))
}

fn hms(secs: i64) -> String {
    format!("{:02}:{:02}:{:02}", secs / 3600, (secs / 60) % 60, secs % 60)
}

fn stamp(t: i64, offset_min: i32) -> String {
    let local = civil(t + i64::from(offset_min) * 60).3;
    format!("{} ({}Z)", hms(local), hms(t.rem_euclid(86_400)))
}

fn when(now: i64, t: i64, offset_min: i32) -> String {
    if now - t < 20 * 3_600 {
        stamp(t, offset_min)
    } else {
        date_time(t, offset_min)[..16].to_string()
    }
}

fn date_time(t: i64, offset_min: i32) -> String {
    let (y, m, d, s) = civil(t + i64::from(offset_min) * 60);
    format!("{y}-{m:02}-{d:02} {}", hms(s))
}

fn date(t: i64) -> String {
    let (y, m, d, _) = civil(t);
    format!("{y}-{m:02}-{d:02}")
}

fn utc_label(offset_min: i32) -> String {
    let sign = if offset_min < 0 { '-' } else { '+' };
    let (h, m) = (offset_min.abs() / 60, offset_min.abs() % 60);
    if m == 0 {
        format!("UTC{sign}{h}")
    } else {
        format!("UTC{sign}{h}:{m:02}")
    }
}

fn ago(secs: i64) -> String {
    match secs {
        s if s < 60 => format!("{} s ago", s.max(0)),
        s if s < 3600 => format!("{} min ago", s / 60),
        s if s < 86_400 => format!("{} h ago", s / 3600),
        s => format!("{} d ago", s / 86_400),
    }
}

fn duration(secs: i64) -> String {
    match secs.max(0) {
        s if s < 60 => format!("{s} s"),
        s if s < 3600 => format!("{} min", s / 60),
        s if s < 86_400 => format!("{} h {} min", s / 3600, (s / 60) % 60),
        s => format!("{} d {} h", s / 86_400, (s / 3600) % 24),
    }
}

fn rarity_name(r: u8) -> &'static str {
    match r {
        1 => "Common",
        2 => "Uncommon",
        3 => "Rare",
        4 => "Epic",
        _ => "Legendary",
    }
}

#[derive(Serialize, Clone, serde::Deserialize)]
pub struct Crash {
    at: i64,
    version: String,
    message: String,
}

fn crash_file() -> Option<std::path::PathBuf> {
    DATA_DIR.get().map(|d| d.join("last_crash.json"))
}

pub fn init(app: &AppHandle) {
    STARTED.store(feed::now(), Ordering::SeqCst);
    if let Ok(dir) = app.path().app_local_data_dir() {
        let _ = DATA_DIR.set(dir);
    }
    let version = app.package_info().version.to_string();
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let message = format!("{info}");
        push(format!("[panic] {message}"));
        if let Some(p) = crash_file() {
            let crash = Crash { at: feed::now(), version: version.clone(), message };
            if let (Some(dir), Ok(json)) = (p.parent(), serde_json::to_string_pretty(&crash)) {
                let _ = std::fs::create_dir_all(dir);
                let tmp = p.with_extension("json.tmp");
                if std::fs::write(&tmp, json).is_ok() {
                    let _ = std::fs::rename(&tmp, &p);
                }
            }
        }
        previous(info);
    }));
}

fn last_crash() -> Option<Crash> {
    crash_file()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
}

fn install_id(app: &AppHandle) -> String {
    if let Some(id) = tray::load_value(app, "install_id").and_then(|v| v.as_str().map(str::to_string)) {
        return id;
    }
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    std::time::SystemTime::now().hash(&mut h);
    std::process::id().hash(&mut h);
    DATA_DIR.get().hash(&mut h);
    let id = format!("{:016x}", h.finish());
    tray::save_values(app, &[("install_id", id.clone().into())]);
    id
}

fn windows_version() -> String {
    use winreg::enums::HKEY_LOCAL_MACHINE;
    use winreg::RegKey;
    let Ok(key) = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion") else {
        return "Windows (unknown)".into();
    };
    let name: String = key.get_value("ProductName").unwrap_or_else(|_| "Windows".into());
    let build: String = key.get_value("CurrentBuild").unwrap_or_default();
    let display: String = key.get_value("DisplayVersion").unwrap_or_default();
    let name = match build.parse::<u32>() {
        Ok(b) if b >= 22_000 => name.replace("Windows 10", "Windows 11"),
        _ => name,
    };
    format!("{name} {display} (build {build})").replace("  ", " ")
}

#[derive(Serialize, Clone)]
pub struct DisplayInfo {
    pub name: Option<String>,
    pub width: u32,
    pub height: u32,
    pub scale: f64,
    pub primary: bool,
}

fn displays(app: &AppHandle) -> Vec<DisplayInfo> {
    let primary = app.primary_monitor().ok().flatten().map(|m| (m.position().x, m.position().y));
    app.available_monitors()
        .unwrap_or_default()
        .into_iter()
        .map(|m| DisplayInfo {
            name: m.name().cloned(),
            width: m.size().width,
            height: m.size().height,
            scale: m.scale_factor(),
            primary: Some((m.position().x, m.position().y)) == primary,
        })
        .collect()
}

fn memory() -> Option<(u64, u64)> {
    #[repr(C)]
    #[derive(Default)]
    struct Counters {
        cb: u32,
        page_fault_count: u32,
        peak_working_set_size: usize,
        working_set_size: usize,
        quota_peak_paged_pool_usage: usize,
        quota_paged_pool_usage: usize,
        quota_peak_non_paged_pool_usage: usize,
        quota_non_paged_pool_usage: usize,
        pagefile_usage: usize,
        peak_pagefile_usage: usize,
        private_usage: usize,
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn GetCurrentProcess() -> isize;
        fn K32GetProcessMemoryInfo(process: isize, counters: *mut Counters, cb: u32) -> i32;
    }
    let mut c = Counters { cb: std::mem::size_of::<Counters>() as u32, ..Default::default() };
    let ok = unsafe { K32GetProcessMemoryInfo(GetCurrentProcess(), &mut c, c.cb) };
    (ok != 0).then_some((c.working_set_size as u64, c.private_usage as u64))
}

#[derive(Serialize, Clone)]
pub struct HelperInfo {
    pub started_at: i64,
    pub started_by: String,
    pub working_set_mb: Option<u64>,
    pub private_mb: Option<u64>,
    pub webview2: Option<String>,
}

fn helper_info() -> HelperInfo {
    let mem = memory();
    HelperInfo {
        started_at: STARTED.load(Ordering::SeqCst),
        started_by: if crate::launch::is_autostart() { "windows" } else { "user" }.to_string(),
        working_set_mb: mem.map(|(w, _)| w / (1024 * 1024)),
        private_mb: mem.map(|(_, p)| p / (1024 * 1024)),
        webview2: tauri::webview_version().ok(),
    }
}

#[derive(Serialize, Clone)]
pub struct GameInfo {
    pub build: Option<u64>,
    pub update_pending: Option<u64>,
    pub updated_at: Option<i64>,
    pub running: bool,
    pub display_mode: Option<DisplayMode>,
}

fn game_info() -> GameInfo {
    GameInfo {
        build: game::build_id(),
        update_pending: game::pending_build(),
        updated_at: game::last_updated(),
        running: crate::process::game_running(),
        display_mode: game::display_mode(),
    }
}

#[derive(Serialize, Clone)]
pub struct SlotInfo {
    pub exists: bool,
    pub bytes: u64,
    pub age_secs: Option<i64>,
    pub ours: bool,
}

fn slot_info() -> SlotInfo {
    let p = paths::slot_dir().join(slot::SLOT_FILE);
    let Ok(meta) = std::fs::metadata(&p) else {
        return SlotInfo { exists: false, bytes: 0, age_secs: None, ours: false };
    };
    let age_secs = meta
        .modified()
        .ok()
        .and_then(|m| m.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| feed::now() - d.as_secs() as i64);
    let ours = std::fs::File::open(&p)
        .ok()
        .and_then(|mut f| {
            use std::io::Read;
            let mut head = [0u8; 4];
            f.read_exact(&mut head).ok().map(|_| &head == b"GVAS")
        })
        .unwrap_or(false);
    SlotInfo { exists: true, bytes: meta.len(), age_secs, ours }
}

#[derive(Serialize, Clone)]
pub struct BadgeFile {
    pub path: String,
    pub crc32: Option<String>,
    pub modified: Option<i64>,
}

fn badge_files() -> Vec<BadgeFile> {
    let Some(root) = game::paks_dir() else { return Vec::new() };
    game::mod_files()
        .into_iter()
        .map(|f| {
            let p = root.join(&f.path);
            let crc32 = std::fs::File::open(&p).ok().and_then(|mut file| {
                use std::io::Read;
                let mut h = crc32fast::Hasher::new();
                let mut buf = [0u8; 64 * 1024];
                loop {
                    match file.read(&mut buf) {
                        Ok(0) => break Some(format!("{:08x}", h.finalize())),
                        Ok(n) => h.update(&buf[..n]),
                        Err(_) => break None,
                    }
                }
            });
            let modified = std::fs::metadata(&p)
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(|m| m.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64);
            BadgeFile { path: f.path, crc32, modified }
        })
        .collect()
}

fn other_mods() -> Vec<String> {
    fn walk(root: &std::path::Path, dir: &std::path::Path, depth: usize, out: &mut BTreeSet<String>) {
        let Ok(entries) = std::fs::read_dir(dir) else { return };
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(kind) = entry.file_type() else { continue };
            if kind.is_dir() {
                if depth < 4 {
                    walk(root, &path, depth + 1, out);
                }
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
            if !(name.ends_with(".pak") || name.ends_with(".utoc") || name.ends_with(".ucas")) || name.contains("murating") {
                continue;
            }
            if depth == 0 && (name.starts_with("pakchunk") || name.starts_with("global.")) {
                continue;
            }
            let rel = path.strip_prefix(root).unwrap_or(&path).with_extension("");
            out.insert(rel.to_string_lossy().replace('\\', "/"));
        }
    }
    let Some(root) = game::paks_dir() else { return Vec::new() };
    let mut out = BTreeSet::new();
    walk(&root, &root, 0, &mut out);
    out.into_iter().collect()
}

#[derive(Serialize, Clone)]
pub struct Status {
    pub mod_files: Vec<ModFile>,
    pub mod_folder: Option<String>,
    pub report_configured: bool,
    pub report_cooldown_secs: i64,
}

pub fn status(app: &AppHandle) -> Status {
    let mod_files = game::mod_files();
    let mod_folder = mod_files
        .iter()
        .find(|f| f.path.ends_with(".pak"))
        .or(mod_files.first())
        .map(|f| f.path.rsplit_once('/').map(|(d, _)| format!("Paks/{d}")).unwrap_or_else(|| "Paks".into()));
    Status {
        mod_folder,
        mod_files,
        report_configured: report::endpoint().is_some(),
        report_cooldown_secs: cooldown_left(app),
    }
}

fn cooldown_left(app: &AppHandle) -> i64 {
    let last = tray::load_value(app, "last_report_at").and_then(|v| v.as_i64());
    report::cooldown_left(last, feed::now())
}

#[derive(Serialize, Clone)]
pub struct FollowInfo {
    pub state: String,
    pub account: Option<String>,
    pub tekken_id: Option<String>,
    pub error: Option<String>,
    pub saved_accounts: usize,
}

fn follow_info(app: &AppHandle) -> FollowInfo {
    use follow::FollowState::*;
    let (state, account, tekken_id, error) = match follow::state() {
        Setup => ("setup", None, None, None),
        Manual => ("manual", None, None, None),
        Paused => ("paused", None, None, None),
        Waiting { expected } => ("waiting", expected, None, None),
        Unpaired => ("unpaired", None, None, None),
        Connecting { tekken_id, name } => ("connecting", Some(name), Some(tekken_id), None),
        Following { me, .. } => ("following", Some(me.name), Some(me.tekken_id), None),
        Failed { tekken_id, name, error, .. } => ("failed", Some(name), Some(tekken_id), Some(error)),
    };
    FollowInfo { state: state.to_string(), account, tekken_id, error, saved_accounts: crate::accounts::load(app).len() }
}

#[derive(Serialize, Clone)]
pub struct CentralInfo {
    pub host: Option<String>,
    pub newest_at: Option<i64>,
    pub behind_secs: Option<i64>,
    pub loading: bool,
    pub clock_skew_secs: Option<i64>,
}

fn central_info(app: &AppHandle) -> CentralInfo {
    let host = feed::central_feed_url(app).map(|u| {
        let rest = u.split_once("://").map(|(_, r)| r).unwrap_or(&u);
        rest.split('/').next().unwrap_or(rest).to_string()
    });
    let newest_at = feed::last_central_at();
    CentralInfo {
        host,
        newest_at,
        behind_secs: newest_at.map(|t| feed::now() - t),
        loading: feed::is_central_loading(),
        clock_skew_secs: SKEW.lock().ok().and_then(|s| *s),
    }
}

#[derive(Serialize, Clone)]
pub struct TuningInfo {
    pub values: std::collections::BTreeMap<String, String>,
    pub keys: usize,
    pub leftovers: Vec<String>,
}

fn tuning_info(app: &AppHandle) -> TuningInfo {
    let all = feed::load_tuning(app);
    let keys = all.len();
    let mut values = std::collections::BTreeMap::new();
    let mut tables: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    let mut leftovers = Vec::new();
    let nonzero = |v: &str| v.parse::<f64>().map_or(true, |n| n != 0.0);
    for (k, v) in all {
        if k.contains('@') && (k.starts_with("me_") || k.starts_with("est@")) {
            *tables.entry(format!("{}@*", k.split('@').next().unwrap_or(&k))).or_default() += 1;
            continue;
        }
        if ((k == "row_test" || k == "delta_late") && nonzero(&v)) || k.starts_with("me_") {
            leftovers.push(format!("{k}={v}"));
        }
        values.insert(k, v);
    }
    leftovers.extend(tables.into_iter().map(|(t, n)| format!("{t} ({n} keys)")));
    TuningInfo { values, keys, leftovers }
}

#[derive(Serialize, Clone)]
pub struct AccountCheck {
    pub linked_to_steam: Option<bool>,
    pub steam_running: bool,
    pub signed_in_matches: Option<bool>,
    pub save_folder_found: bool,
}

fn account_check(app: &AppHandle) -> AccountCheck {
    let steam_running = crate::accounts::steam_up();
    let followed = tray::current_tekken_id(app)
        .and_then(|id| crate::accounts::load(app).into_iter().find(|a| a.tekken_id == id));
    let signed_in = murating_core::steam::active_steam_id();
    AccountCheck {
        linked_to_steam: followed.as_ref().map(|a| a.steam_id.is_some()),
        steam_running,
        signed_in_matches: followed
            .as_ref()
            .map(|a| steam_running && a.steam_id.is_some() && a.steam_id == signed_in),
        save_folder_found: paths::active_account(&paths::saves_root()).is_ok(),
    }
}

#[derive(Serialize, Clone)]
pub struct MrRow {
    pub character: String,
    pub code: Option<String>,
    pub helper: i32,
    pub wavu: Option<i32>,
    pub asof: Option<i64>,
    pub rd: Option<f64>,
    pub status: String,
}

#[derive(Serialize, Clone)]
pub struct MrReport {
    pub badge_text: Option<String>,
    pub page_read_at: Option<i64>,
    pub rows: Vec<MrRow>,
}

fn mr_report(app: &AppHandle, feed: Option<&feed::DiagFeed>) -> MrReport {
    let page_read_at = feed.and_then(|f| f.page_fetched_at);
    let rows = feed
        .map(|f| {
            f.characters
                .iter()
                .map(|c| {
                    let status = match c.page_mu {
                        None => "not_on_page",
                        Some(w) if w == c.base => "same",
                        Some(_) if c.asof.zip(page_read_at).is_some_and(|(asof, read)| asof > read) => "ahead",
                        Some(_) => "mismatch",
                    };
                    MrRow {
                        character: c.character.clone(),
                        code: c.code.clone(),
                        helper: c.base,
                        wavu: c.page_mu,
                        asof: c.asof,
                        rd: c.rd,
                        status: status.to_string(),
                    }
                })
                .collect()
        })
        .unwrap_or_default();
    MrReport { badge_text: tray::payload(app), page_read_at, rows }
}

#[derive(Serialize, Clone)]
pub struct LastAward {
    pub title: String,
    pub rarity: u8,
    pub kind: String,
    pub at: i64,
}

#[derive(Serialize, Clone)]
pub struct AchSummary {
    pub earned: usize,
    pub total: usize,
    pub unread: usize,
    pub last_award: Option<LastAward>,
}

fn ach_summary(app: &AppHandle) -> Option<AchSummary> {
    use murating_core::achievements::{FeedItem, Kind};
    let s = crate::achievements::snapshot(app)?;
    let last_award = s.feed.iter().find_map(|f| match f {
        FeedItem::Award { title, rarity, entry_kind, at, .. } => Some(LastAward {
            title: title.clone(),
            rarity: *rarity,
            kind: if matches!(entry_kind, Kind::Milestone) { "milestone" } else { "achievement" }.to_string(),
            at: *at,
        }),
        FeedItem::Battle { .. } => None,
    });
    Some(AchSummary { earned: s.earned, total: s.total, unread: s.unread, last_award })
}

#[derive(Serialize, Clone)]
pub struct LogLine {
    pub at: i64,
    pub text: String,
}

#[derive(Serialize, Clone)]
pub struct Report {
    pub schema: u32,
    pub sent_at: i64,
    pub utc_offset_min: i32,
    pub install_id: String,
    pub app_version: String,
    pub warnings: Vec<String>,
    pub helper: HelperInfo,
    pub windows: String,
    pub displays: Vec<DisplayInfo>,
    pub game: GameInfo,
    pub mod_files: Vec<ModFile>,
    pub badge_build: Vec<BadgeFile>,
    pub other_mods: Vec<String>,
    pub slot: SlotInfo,
    pub settings: tray::Settings,
    pub follow: FollowInfo,
    pub account_check: AccountCheck,
    pub wavu: NetStat,
    pub central_feed: CentralInfo,
    pub feed_requests: NetStat,
    pub feed: Option<feed::DiagFeed>,
    pub mr: MrReport,
    pub session: Option<murating_core::session::SessionSummary>,
    pub achievements: Option<AchSummary>,
    pub tuning: TuningInfo,
    pub last_crash: Option<Crash>,
    pub errors: Vec<LogLine>,
    pub log: Vec<LogLine>,
    pub note: Option<String>,
}

fn gather(app: &AppHandle, note: Option<String>) -> Report {
    let feed_snapshot = feed::diag_feed();
    let log = log_lines();
    let errors: Vec<LogLine> = log.iter().filter(|l| is_error(&l.text)).cloned().collect();
    let mut r = Report {
        schema: 3,
        sent_at: feed::now(),
        utc_offset_min: crate::achievements::utc_offset(app),
        install_id: install_id(app),
        app_version: app.package_info().version.to_string(),
        warnings: Vec::new(),
        helper: helper_info(),
        windows: windows_version(),
        displays: displays(app),
        game: game_info(),
        mod_files: game::mod_files(),
        badge_build: badge_files(),
        other_mods: other_mods(),
        slot: slot_info(),
        settings: tray::settings(app),
        follow: follow_info(app),
        account_check: account_check(app),
        wavu: net(Endpoint::Wavu),
        central_feed: central_info(app),
        feed_requests: net(Endpoint::Feed),
        mr: mr_report(app, feed_snapshot.as_ref()),
        feed: feed_snapshot,
        session: crate::feed::session(app),
        achievements: ach_summary(app),
        tuning: tuning_info(app),
        last_crash: last_crash(),
        errors,
        log,
        note: note.map(|n| n.chars().take(2000).collect::<String>()),
    };
    r.warnings = warnings(&r);
    r
}

fn warnings(r: &Report) -> Vec<String> {
    let now = r.sent_at;
    let mut w = Vec::new();
    if r.mod_files.is_empty() {
        w.push("The HUD mod was not found in the game's Paks folder, so the badge can't show.".to_string());
    }
    let mod_made = r.badge_build.iter().filter_map(|b| b.modified).max();
    if let (Some(updated), Some(made)) = (r.game.updated_at, mod_made) {
        if updated > made + 3600 {
            w.push(format!(
                "The game was updated on {} after the mod files were made ({}): the mod may need an update.",
                date(updated),
                date(made)
            ));
        }
    }
    if let Some(p) = r.game.update_pending {
        w.push(format!("A Tekken update (build {p}) is waiting in Steam."));
    }
    if r.slot.exists && !r.slot.ours {
        w.push("MuRating.sav is not a MuRating save slot.".to_string());
    }
    if r.follow.state == "following" && !r.slot.exists {
        w.push("Connected, but the save slot has never been written.".to_string());
    }
    match r.follow.state.as_str() {
        "following" => {}
        "failed" => w.push(format!("Could not connect: {}", r.follow.error.as_deref().unwrap_or("no reason given"))),
        other => w.push(format!("Not connected ({other}).")),
    }
    if r.account_check.steam_running && r.account_check.signed_in_matches == Some(false) {
        w.push("Steam is signed in to a different account than the one the helper follows.".to_string());
    }
    if let Some(behind) = r.central_feed.behind_secs {
        if behind > 8 * 3600 {
            w.push(format!("The central feed is {} behind: opponents will show no MR.", duration(behind)));
        } else if behind > 900 {
            w.push(format!("The central feed is {} behind.", duration(behind)));
        }
    }
    for (name, s) in [("central feed", &r.feed_requests), ("Wavu Wank", &r.wavu)] {
        if let (Some(at), Some(e)) = (s.last_error_at, &s.last_error) {
            if now - at < 1800 && s.last_ok_at.map_or(true, |ok| ok < at) {
                w.push(format!("The last request to the {name} failed {}: {e}", ago(now - at)));
            }
        }
    }
    if let Some(skew) = r.central_feed.clock_skew_secs {
        if skew.abs() > 60 {
            w.push(format!(
                "This PC's clock is {} {} the feed server's.",
                duration(skew.abs()),
                if skew > 0 { "ahead of" } else { "behind" }
            ));
        }
    }
    if let Some(e) = r.feed.as_ref().and_then(|f| f.last_write_error.as_ref()) {
        w.push(format!("The last save slot write failed: {e}"));
    }
    let mismatched: Vec<String> = r
        .mr
        .rows
        .iter()
        .filter(|m| m.status == "mismatch")
        .map(|m| format!("{} (helper {}, Wavu {})", m.character, m.helper, m.wavu.unwrap_or_default()))
        .collect();
    if !mismatched.is_empty() {
        w.push(format!("The helper's MR and Wavu's disagree on {}.", mismatched.join(", ")));
    }
    if r.game.display_mode.as_ref().is_some_and(|d| d.mode == "fullscreen") {
        w.push("Tekken is in exclusive fullscreen, where notifications can't draw over the game.".to_string());
    }
    if r.helper.webview2.is_none() {
        w.push("The WebView2 runtime was not found.".to_string());
    }
    if !r.tuning.leftovers.is_empty() {
        w.push(format!("tuning.json has test settings left in it: {}.", r.tuning.leftovers.join(", ")));
    }
    if let Some(c) = &r.last_crash {
        if now - c.at < 7 * 86_400 {
            w.push(format!("The helper crashed {} ({}).", ago(now - c.at), c.version));
        }
    }
    w
}

pub fn build(app: &AppHandle, note: Option<String>) -> serde_json::Value {
    serde_json::to_value(gather(app, note)).unwrap_or(serde_json::Value::Null)
}

pub fn text(app: &AppHandle) -> String {
    render(&gather(app, None))
}

pub fn render(r: &Report) -> String {
    let now = r.sent_at;
    let off = r.utc_offset_min;
    let mut out = String::new();
    let line = |out: &mut String, label: &str, value: String| out.push_str(&format!("{label:<15}{value}\n"));
    let head = |out: &mut String, title: &str| out.push_str(&format!("\n{title}\n"));
    let yn = |b: bool| if b { "yes" } else { "no" };
    let on = |b: bool| if b { "on" } else { "off" };
    let at = |t: Option<i64>| t.map(|t| ago(now - t)).unwrap_or_else(|| "never".into());

    out.push_str("Mu Rating HUD diagnostics\n");
    line(&mut out, "Copied", format!("{} local · {}Z · {}", date_time(now, off), hms(now.rem_euclid(86_400)), utc_label(off)));

    head(&mut out, "SUMMARY");
    line(
        &mut out,
        "Version",
        format!(
            "{} · started by {} {} ago",
            r.app_version,
            if r.helper.started_by == "windows" { "Windows" } else { "the player" },
            duration(now - r.helper.started_at)
        ),
    );
    let f = &r.follow;
    let who = match (&f.account, &f.tekken_id) {
        (Some(a), Some(id)) => format!(" {a} ({id})"),
        (Some(a), None) => format!(" {a}"),
        _ => String::new(),
    };
    let connection = format!("{}{who}{}", f.state, f.error.as_ref().map(|e| format!(": {e}")).unwrap_or_default());
    line(&mut out, "Connection", connection.clone());
    if r.warnings.is_empty() {
        line(&mut out, "Warnings", "none".into());
    } else {
        line(&mut out, "Warnings", r.warnings.len().to_string());
        for w in &r.warnings {
            out.push_str(&format!("  - {w}\n"));
        }
    }

    head(&mut out, "SYSTEM");
    line(&mut out, "Windows", r.windows.clone());
    let screens: Vec<String> = r
        .displays
        .iter()
        .map(|d| {
            format!(
                "{}x{} at {:.0}%{}",
                d.width,
                d.height,
                d.scale * 100.0,
                if d.primary { " (primary)" } else { "" }
            )
        })
        .collect();
    line(
        &mut out,
        "Displays",
        if screens.is_empty() { "unknown".into() } else { format!("{} · {}", screens.len(), screens.join(" · ")) },
    );
    line(&mut out, "WebView2", r.helper.webview2.clone().unwrap_or_else(|| "not found".into()));
    line(
        &mut out,
        "Helper memory",
        match (r.helper.private_mb, r.helper.working_set_mb) {
            (Some(p), Some(w)) => format!("{p} MB private, {w} MB working set (WebView2 not counted)"),
            _ => "unknown".into(),
        },
    );

    head(&mut out, "GAME AND MOD");
    let g = &r.game;
    line(
        &mut out,
        "Game build",
        format!(
            "{}{}{}{}",
            g.build.map(|b| b.to_string()).unwrap_or_else(|| "not found".into()),
            g.updated_at.map(|t| format!(" · updated {}", date(t))).unwrap_or_default(),
            g.update_pending.map(|p| format!(" · update {p} pending")).unwrap_or_default(),
            if g.running { " · running" } else { " · not running" }
        ),
    );
    line(
        &mut out,
        "Tekken display",
        g.display_mode
            .as_ref()
            .map(|d| match (d.width, d.height) {
                (Some(w), Some(h)) => format!("{} {w}x{h}", d.mode),
                _ => d.mode.clone(),
            })
            .unwrap_or_else(|| "settings file not found".into()),
    );
    line(
        &mut out,
        "Mod files",
        if r.mod_files.is_empty() {
            "not found in Paks".into()
        } else {
            r.mod_files.iter().map(|m| m.path.clone()).collect::<Vec<_>>().join(", ")
        },
    );
    if let Some(b) = r.badge_build.iter().find(|b| b.path.ends_with(".ucas")).or(r.badge_build.first()) {
        line(
            &mut out,
            "Badge build",
            format!(
                "{} crc {} · modified {}{}",
                b.path.rsplit('/').next().unwrap_or(&b.path),
                b.crc32.as_deref().unwrap_or("unreadable"),
                b.modified.map(date).unwrap_or_else(|| "?".into()),
                b.modified.map(|m| format!(" ({})", ago(now - m))).unwrap_or_default()
            ),
        );
    }
    line(
        &mut out,
        "Other mods",
        if r.other_mods.is_empty() { "none".into() } else { format!("{} · {}", r.other_mods.len(), r.other_mods.join(", ")) },
    );
    let s = &r.slot;
    line(
        &mut out,
        "Save slot",
        if s.exists {
            format!(
                "{} KB, written {}{}",
                s.bytes / 1024,
                s.age_secs.map(ago).unwrap_or_else(|| "?".into()),
                if s.ours { "" } else { ", NOT a MuRating slot" }
            )
        } else {
            "not written yet".into()
        },
    );

    head(&mut out, "CONNECTION");
    line(&mut out, "Connection", connection);
    let a = &r.account_check;
    line(
        &mut out,
        "Account check",
        format!(
            "Steam running {} · linked to Steam {} · signed in as this account {} · save folder found {}",
            yn(a.steam_running),
            a.linked_to_steam.map(yn).unwrap_or("-"),
            a.signed_in_matches.map(yn).unwrap_or("-"),
            yn(a.save_folder_found)
        ),
    );
    let requests = |n: &NetStat| {
        let mut parts = vec![n.last_ok_at.map(|t| format!("last answered {}", ago(now - t))).unwrap_or_else(|| "no answer yet".into())];
        if let Some(ms) = n.last_ms {
            parts.push(format!("{ms} ms"));
        }
        parts.push(format!("{} requests", n.requests));
        parts.push(match (n.errors, &n.last_error, n.last_error_at) {
            (0, _, _) => "no errors".into(),
            (1, Some(e), Some(t)) => format!("1 error, {}: {e}", ago(now - t)),
            (count, Some(e), Some(t)) => format!("{count} errors, last {}: {e}", ago(now - t)),
            (count, _, _) => format!("{count} errors"),
        });
        parts.join(" · ")
    };
    line(&mut out, "Wavu", requests(&r.wavu));
    let c = &r.central_feed;
    line(
        &mut out,
        "Central feed",
        format!(
            "{}{}{}{}",
            c.host.as_deref().unwrap_or("not configured"),
            match c.behind_secs {
                Some(b) if b > 8 * 3600 => format!(" · newest record {} - PAST THE 8 h NAME HORIZON", ago(b)),
                Some(b) if b > 900 => format!(" · newest record {} - falling behind", ago(b)),
                Some(b) => format!(" · newest record {}", ago(b)),
                None => " · no records yet".into(),
            },
            r.feed
                .as_ref()
                .map(|fd| format!(" · {} hours, {} minutes merged", fd.central_hours_loaded, fd.central_minutes_merged))
                .unwrap_or_default(),
            if c.loading { " · still loading" } else { "" }
        ),
    );
    line(&mut out, "Feed requests", requests(&r.feed_requests));
    line(
        &mut out,
        "Clock",
        match c.clock_skew_secs {
            None => "not checked yet".into(),
            Some(sk) if sk.abs() <= 5 => format!("matches the feed server ({sk:+} s)"),
            Some(sk) => format!("{} {} the feed server", duration(sk.abs()), if sk > 0 { "ahead of" } else { "behind" }),
        },
    );
    if let Some(fd) = &r.feed {
        line(&mut out, "Store", format!("{} records · {} players · {} deviations", fd.records, fd.players, fd.rd_entries));
        line(
            &mut out,
            "Timing",
            format!(
                "page fetched {} · replay save {} · last held back in game {}",
                at(fd.page_fetched_at),
                at(fd.last_replay_save_at),
                at(fd.last_deferred_at)
            ),
        );
        if let Some(wr) = &fd.last_slot_write {
            line(
                &mut out,
                "Last write",
                format!(
                    "{}{}, {} KB: {} rows ({} min), {} names ({:.1} h){}, built {} ms, written {} ms",
                    ago(now - wr.at),
                    if wr.forced { ", forced" } else { "" },
                    wr.bytes / 1024,
                    wr.replay_rows,
                    wr.replay_rows_minutes,
                    wr.names,
                    wr.name_hours,
                    if wr.trimmed { ", TRIMMED BY THE 4 MB CAP" } else { "" },
                    wr.build_ms,
                    wr.write_ms
                ),
            );
        }
        if let Some(e) = &fd.last_write_error {
            line(&mut out, "Write error", e.clone());
        }
    }

    head(&mut out, "REPORTED MR (helper vs Wavu)");
    line(&mut out, "Badge shows", r.mr.badge_text.clone().unwrap_or_else(|| "nothing yet".into()));
    line(&mut out, "Wavu page", r.mr.page_read_at.map(|t| format!("read {}", ago(now - t))).unwrap_or_else(|| "not read yet".into()));
    if r.mr.rows.is_empty() {
        out.push_str("  no ratings yet\n");
    }
    for m in &r.mr.rows {
        let name = format!("{}{}", m.character, m.code.as_deref().map(|c| format!(" ({c})")).unwrap_or_default());
        let verdict = match (m.status.as_str(), m.wavu) {
            ("same", _) => "same".to_string(),
            ("ahead", Some(w)) => format!("helper {:+} from battles after the page was read", m.helper - w),
            ("mismatch", Some(w)) => format!("MISMATCH ({:+})", m.helper - w),
            _ => "not on the Wavu page yet".to_string(),
        };
        out.push_str(&format!(
            "  {name:<20} helper {:<5} Wavu {:<5} {verdict}{}{}\n",
            m.helper,
            m.wavu.map(|w| w.to_string()).unwrap_or_else(|| "-".into()),
            m.asof.map(|t| format!(" · as of {}", when(now, t, off))).unwrap_or_default(),
            m.rd.map(|rd| format!(" · RD {rd:.0}")).unwrap_or_default()
        ));
    }
    if let Some(fd) = &r.feed {
        if !fd.recent_matches.is_empty() {
            out.push_str("Recent matches\n");
            for m in &fd.recent_matches {
                let result = match (m.change, m.mr_after) {
                    (Some(ch), Some(after)) => format!("{ch:+} -> {after}"),
                    _ => "not rated yet".into(),
                };
                out.push_str(&format!("  {} {} {result}\n", stamp(m.at, off), m.character.as_deref().unwrap_or("?")));
            }
        }
    }

    head(&mut out, "SESSION AND ACHIEVEMENTS");
    line(
        &mut out,
        "Session",
        r.session
            .as_ref()
            .map(|se| {
                format!(
                    "since {} · {} W {} L · {:+} MR · streak {}",
                    stamp(se.since, off),
                    se.wins,
                    se.losses,
                    se.net,
                    match se.streak {
                        0 => "none".to_string(),
                        n if n > 0 => format!("{n} {}", if n == 1 { "win" } else { "wins" }),
                        n => format!("{} {}", -n, if n == -1 { "loss" } else { "losses" }),
                    }
                )
            })
            .unwrap_or_else(|| "none".into()),
    );
    line(
        &mut out,
        "Achievements",
        r.achievements
            .as_ref()
            .map(|ac| {
                format!(
                    "{} of {} · {} unread{}",
                    ac.earned,
                    ac.total,
                    ac.unread,
                    ac.last_award
                        .as_ref()
                        .map(|l| format!(" · last: {} ({} {}) {}", l.title, rarity_name(l.rarity), l.kind, ago(now - l.at)))
                        .unwrap_or_default()
                )
            })
            .unwrap_or_else(|| "none".into()),
    );

    head(&mut out, "SETTINGS");
    let st = &r.settings;
    line(
        &mut out,
        "Settings",
        format!(
            "MR {} · auto-connect {} · auto-hide {} · autostart {} · toasts {} · over game {} · headless {}",
            on(st.mr_enabled),
            on(st.auto_connect),
            on(st.auto_hide),
            on(st.start_with_windows),
            on(!st.hide_notifications),
            on(st.over_game),
            on(st.headless)
        ),
    );
    let t = &r.tuning;
    line(
        &mut out,
        "Tuning",
        if t.keys == 0 {
            "no tuning.json".into()
        } else if t.leftovers.is_empty() {
            format!("{} keys", t.keys)
        } else {
            format!("{} keys · TEST LEFTOVERS: {}", t.keys, t.leftovers.join(", "))
        },
    );
    line(
        &mut out,
        "Last crash",
        r.last_crash
            .as_ref()
            .map(|cr| format!("{} ({}): {}", stamp(cr.at, off), cr.version, cr.message.lines().next().unwrap_or("")))
            .unwrap_or_else(|| "none".into()),
    );

    let errors: Vec<&LogLine> = r.errors.iter().rev().take(ERROR_LINES).rev().collect();
    head(&mut out, &format!("ERRORS (last {ERROR_LINES})"));
    if errors.is_empty() {
        out.push_str("  none\n");
    }
    for l in errors {
        out.push_str(&format!("  {} {}\n", stamp(l.at, off), l.text));
    }
    head(&mut out, &format!("LOG (last {LOG_LINES})"));
    for l in r.log.iter().rev().take(LOG_LINES).rev() {
        out.push_str(&format!("  {} {}\n", stamp(l.at, off), l.text));
    }
    out
}

#[derive(Serialize)]
pub struct Sent {
    pub sent: bool,
    pub saved: Option<String>,
    pub cooldown_secs: i64,
}

pub fn send(app: &AppHandle, note: Option<String>) -> Result<Sent, String> {
    let left = cooldown_left(app);
    if left > 0 {
        return Err(format!("You can send another report in {} min.", (left + 59) / 60));
    }
    let body = build(app, note);
    let json = serde_json::to_string_pretty(&body).map_err(|e| e.to_string())?;

    let saved = app.path().app_local_data_dir().ok().and_then(|dir| {
        let dir = dir.join("reports");
        std::fs::create_dir_all(&dir).ok()?;
        let p = dir.join(format!("report-{}.json", feed::now()));
        let tmp = p.with_extension("json.tmp");
        std::fs::write(&tmp, &json).ok()?;
        std::fs::rename(&tmp, &p).ok()?;
        Some(p.to_string_lossy().to_string())
    });

    match report::post(&json, crate::commands::USER_AGENT) {
        Ok(()) => {
            tray::save_values(app, &[("last_report_at", feed::now().into())]);
            push("[report] sent".into());
            Ok(Sent { sent: true, saved, cooldown_secs: report::COOLDOWN_SECS })
        }
        Err(report::ReportError::NotConfigured) => Ok(Sent { sent: false, saved, cooldown_secs: 0 }),
        Err(e) => {
            push(format!("[report] failed: {e}"));
            Err(format!("Could not send the report: {e}"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::feed::{DiagCharacter, DiagFeed, DiagMatch, DiagSlotWrite};

    #[test]
    fn times_read_in_both_clocks() {
        assert_eq!(date_time(1_789_655_999, -240), "2026-09-17 10:39:59");
        assert_eq!(stamp(1_789_655_999, -240), "10:39:59 (14:39:59Z)");
        assert_eq!(stamp(1_789_600_000, 330), "04:36:40 (23:06:40Z)", "across midnight");
        assert_eq!(when(1_789_655_999, 1_789_655_999 - 3 * 86_400, -240), "2026-09-14 10:39");
        assert_eq!(when(1_789_655_999, 1_789_655_999 - 600, -240), "10:29:59 (14:29:59Z)");
        assert_eq!((utc_label(-240), utc_label(330), utc_label(0)), ("UTC-4".into(), "UTC+5:30".into(), "UTC+0".into()));
        for t in [0_i64, 951_782_400, 1_789_655_999, 4_107_542_400] {
            let (y, m, d, s) = civil(t);
            assert_eq!(murating_core::central::days_from_civil(y, m, d) * 86_400 + s, t, "{t}");
        }
    }

    fn status_of(c: &DiagCharacter, page_read_at: Option<i64>) -> &'static str {
        match c.page_mu {
            None => "not_on_page",
            Some(w) if w == c.base => "same",
            Some(_) if c.asof.zip(page_read_at).is_some_and(|(asof, read)| asof > read) => "ahead",
            Some(_) => "mismatch",
        }
    }

    #[test]
    fn a_whole_report_renders() {
        let now = feed::now();
        let off = -240;
        let chars: [(&str, &str, i32, i32, i64, f64); 11] = [
            ("Reina", "zbr", 2527, 2515, 2 * 60, 68.0),
            ("Dragunov", "kmd", 2019, 2019, 2 * 86_400, 73.0),
            ("Devil Jin", "swl", 2242, 2242, 6 * 86_400, 86.0),
            ("Kazuya", "grl", 2224, 2224, 17 * 86_400, 80.0),
            ("Miary Zo", "wkz", 2167, 2167, 21 * 86_400, 103.0),
            ("Lidia", "cbr", 2165, 2165, 40 * 86_400, 102.0),
            ("Heihachi", "bee", 2096, 2096, 16 * 86_400, 87.0),
            ("Armor King", "knk", 2074, 2074, 47 * 86_400, 80.0),
            ("Jin", "ant", 2011, 2011, 22 * 86_400, 81.0),
            ("Kunimitsu", "ker", 2117, 2117, 7 * 86_400, 112.0),
            ("Fahkumram", "tgr", 2095, 2095, 30 * 86_400, 121.0),
        ];
        let feed_snapshot = DiagFeed {
            characters: chars
                .iter()
                .map(|(name, code, helper, wavu, ago, rd)| DiagCharacter {
                    character: name.to_string(),
                    code: Some(code.to_string()),
                    page_mu: Some(*wavu),
                    base: *helper,
                    asof: Some(now - ago),
                    rd: Some(*rd),
                })
                .collect(),
            recent_matches: [(34_i64, 2503, 9), (26, 2502, -1), (17, 2505, 3), (9, 2515, 10), (2, 2527, 12)]
                .iter()
                .map(|(mins, after, change)| DiagMatch {
                    at: now - mins * 60,
                    character: Some("Reina".into()),
                    rated: true,
                    change: Some(*change),
                    mr_after: Some(*after),
                })
                .collect(),
            records: 48_210,
            players: 11_342,
            rd_entries: 11_342,
            page_fetched_at: Some(now - 14 * 60),
            last_slot_write: Some(DiagSlotWrite {
                at: now - 2 * 60,
                forced: true,
                bytes: 3_762_104,
                replay_rows: 1_804,
                replay_rows_minutes: 60,
                names: 21_406,
                name_hours: 8.0,
                trimmed: false,
                build_ms: 212,
                write_ms: 41,
            }),
            last_deferred_at: Some(now - 5 * 60),
            last_replay_save_at: Some(now - 2 * 60),
            central_hours_loaded: 8,
            central_minutes_merged: 131,
            last_write_error: None,
        };
        let log: Vec<LogLine> = [
            (8_040, "[feed] reading the central feed at https://mr.tekkenresourcehub.com"),
            (7_320, "[feed] central minute 29827311: Network Error: timed out reading response"),
            (840, "[feed] own battle newer than the page: refetching for an exact |me_base"),
            (130, "[delta] replay saved at 1789655869"),
            (125, "[delta] battle 1789655874 on zbr (3 rounds, god true): shown Some(2515) -> Some(2527)"),
            (122, "[feed] wrote 3674 KB in 41 ms (built in 212 ms): 1804 rows (60 min), 21406 names (8 h), payload \"2527 MR\""),
        ]
        .iter()
        .map(|(ago, text)| LogLine { at: now - ago, text: text.to_string() })
        .collect();
        let page_read_at = feed_snapshot.page_fetched_at;
        let rows = feed_snapshot
            .characters
            .iter()
            .map(|c| MrRow {
                character: c.character.clone(),
                code: c.code.clone(),
                helper: c.base,
                wavu: c.page_mu,
                asof: c.asof,
                rd: c.rd,
                status: status_of(c, page_read_at).into(),
            })
            .collect();
        let errors = log.iter().filter(|l| is_error(&l.text)).cloned().collect();
        let mut r = Report {
            schema: 3,
            sent_at: now,
            utc_offset_min: off,
            install_id: "3f9c2a71d04be816".into(),
            app_version: "0.1.1".into(),
            warnings: Vec::new(),
            helper: HelperInfo {
                started_at: now - 2 * 3_600 - 13 * 60,
                started_by: "windows".into(),
                working_set_mb: Some(61),
                private_mb: Some(38),
                webview2: tauri::webview_version().ok(),
            },
            windows: windows_version(),
            displays: vec![DisplayInfo { name: Some("DISPLAY1".into()), width: 3840, height: 2160, scale: 2.25, primary: true }],
            game: game_info(),
            mod_files: game::mod_files(),
            badge_build: badge_files(),
            other_mods: other_mods(),
            slot: slot_info(),
            settings: tray::Settings {
                mr_enabled: true,
                auto_hide: true,
                start_with_windows: true,
                hide_notifications: false,
                auto_connect: true,
                fighter_select: true,
                pager_auto_scroll: true,
                over_game: true,
                headless: false,
            },
            follow: FollowInfo {
                state: "following".into(),
                account: Some("HeavenlyUzumaki".into()),
                tekken_id: Some("8wKd2Pq6Rz4M".into()),
                error: None,
                saved_accounts: 5,
            },
            account_check: AccountCheck {
                linked_to_steam: Some(true),
                steam_running: true,
                signed_in_matches: Some(true),
                save_folder_found: true,
            },
            wavu: NetStat { last_ok_at: Some(now - 14 * 60), last_ms: Some(388), last_error: None, last_error_at: None, requests: 3, errors: 0 },
            central_feed: CentralInfo {
                host: Some("mr.tekkenresourcehub.com".into()),
                newest_at: Some(now - 50),
                behind_secs: Some(50),
                loading: false,
                clock_skew_secs: Some(1),
            },
            feed_requests: NetStat {
                last_ok_at: Some(now - 20),
                last_ms: Some(91),
                last_error: Some("minute file 29827311: Network Error: timed out reading response".into()),
                last_error_at: Some(now - 7_320),
                requests: 212,
                errors: 1,
            },
            mr: MrReport { badge_text: Some("2527 MR".into()), page_read_at, rows },
            feed: Some(feed_snapshot),
            session: Some(murating_core::session::SessionSummary {
                since: now - 2 * 3_600,
                net: 80,
                wins: 12,
                losses: 5,
                last8: vec![true, true, false, false, true, true, true, true],
                streak: 5,
                last: None,
                trend: None,
                goal: None,
            }),
            achievements: Some(AchSummary {
                earned: 20,
                total: 75,
                unread: 1,
                last_award: Some(LastAward { title: "Hot Streak".into(), rarity: 2, kind: "achievement".into(), at: now - 2 * 60 }),
            }),
            tuning: TuningInfo { values: Default::default(), keys: 5, leftovers: Vec::new() },
            last_crash: None,
            errors,
            log,
            note: None,
        };
        r.warnings = warnings(&r);
        let text = render(&r);
        println!("{text}");

        for section in ["SUMMARY", "SYSTEM", "GAME AND MOD", "CONNECTION", "REPORTED MR (helper vs Wavu)", "SESSION AND ACHIEVEMENTS", "SETTINGS", "ERRORS", "LOG"] {
            assert!(text.contains(section), "{section}");
        }
        assert!(text.contains("helper +12 from battles after the page was read"), "Reina is ahead of her page");
        assert!(!text.contains("```"), "plain text for a Discord form");
        assert_eq!(r.errors.len(), 1, "the timeout, and only the timeout");
        assert!(serde_json::to_value(&r).is_ok());
    }
}
