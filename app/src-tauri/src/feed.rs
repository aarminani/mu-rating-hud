use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use std::collections::BTreeMap;

use murating_core::delta::Ledger;
use murating_core::feed::{self, Store};
use murating_core::player::{self, HistoryRow, Rating};
use murating_core::replays::{self, Record, WINDOW_SECS};
use murating_core::{paths, slot};

use crate::commands::{char_ratings, CharRating, USER_AGENT};
use crate::tray;

#[derive(Serialize, Clone, PartialEq)]
pub struct RatingsUpdate {
    pub character: String,
    pub mu: i32,
    pub ratings: Vec<CharRating>,
}

const PAGE_REFETCH: i64 = 600;

const OWN_BATTLE_REFETCH: i64 = 600;

const OWN_BATTLE_QUIET: i64 = 300;

fn characters_file(app: &AppHandle) -> Option<std::path::PathBuf> {
    app.path().app_local_data_dir().ok().map(|d| d.join("characters.json"))
}

fn own_power_file(app: &AppHandle) -> Option<std::path::PathBuf> {
    app.path().app_local_data_dir().ok().map(|d| d.join("own_power.json"))
}

fn players_file(app: &AppHandle) -> Option<std::path::PathBuf> {
    app.path().app_local_data_dir().ok().map(|d| d.join("players2.json"))
}

fn seed_rd_from_page(app: &AppHandle, writer: &mut Writer, page: &[Rating], store: &Store) {
    for r in page {
        let (Some(code), Some(sigma)) = (crate::roster::code_for_name(app, &r.character), r.sigma)
        else {
            continue;
        };
        let Some(chara) = writer.chara_of(app, store, &code) else { continue };
        writer.rd.seed(&writer.tekken_id, chara, f64::from(sigma), now());
    }
}

fn rd_file(app: &AppHandle) -> Option<std::path::PathBuf> {
    app.path().app_local_data_dir().ok().map(|d| d.join("rd.json"))
}

fn load_rd(app: &AppHandle) -> murating_core::rd::RdTable {
    rd_file(app)
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_rd(app: &AppHandle, rd: &murating_core::rd::RdTable) {
    let Some(p) = rd_file(app) else { return };
    if let Some(dir) = p.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(json) = serde_json::to_string(rd) {
        let _ = std::fs::write(p, json);
    }
}

fn load_players(app: &AppHandle) -> feed::SavedPlayers {
    players_file(app)
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_players(app: &AppHandle, store: &Store) {
    let Some(p) = players_file(app) else { return };
    if let Some(dir) = p.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let Ok(json) = serde_json::to_string(&store.saved_players(now())) else { return };
    let tmp = p.with_extension("json.tmp");
    if std::fs::write(&tmp, json).is_ok() {
        let _ = std::fs::rename(&tmp, &p);
    }
}

fn ledger_file(app: &AppHandle, tekken_id: &str) -> Option<std::path::PathBuf> {
    app.path().app_local_data_dir().ok().map(|d| d.join(format!("delta_{tekken_id}.json")))
}

fn load_ledger(app: &AppHandle, tekken_id: &str) -> Ledger {
    ledger_file(app, tekken_id)
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_ledger(app: &AppHandle, tekken_id: &str, ledger: &Ledger) {
    let Some(p) = ledger_file(app, tekken_id) else { return };
    if let Some(dir) = p.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(json) = serde_json::to_string_pretty(ledger) {
        let tmp = p.with_extension("json.tmp");
        if std::fs::write(&tmp, json).is_ok() {
            let _ = std::fs::rename(&tmp, &p);
        }
    }
}

fn newest_replay_save() -> Option<i64> {
    let dir = paths::active_account(&paths::saves_root()).ok()?;
    let mut newest: Option<i64> = None;
    for entry in std::fs::read_dir(dir).ok()?.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !(name.starts_with("replay_game_no") && name.ends_with(".sav")) {
            continue;
        }
        let Some(t) = entry
            .metadata()
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
        else {
            continue;
        };
        newest = Some(newest.map_or(t, |n| n.max(t)));
    }
    newest
}

const REPLAY_CHECK: Duration = Duration::from_secs(2);

const PLAYERS_SAVE: i64 = 300;

const BACKFILL_PUBLISH: usize = 15;

fn load_own_power(app: &AppHandle, tekken_id: &str) -> Option<i64> {
    own_power_file(app)
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str::<std::collections::HashMap<String, i64>>(&s).ok())
        .and_then(|m| m.get(tekken_id).copied())
}

fn save_own_power(app: &AppHandle, tekken_id: &str, power: i64) {
    let Some(p) = own_power_file(app) else { return };
    let mut m: std::collections::HashMap<String, i64> = std::fs::read_to_string(&p)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    m.insert(tekken_id.to_string(), power);
    if let Some(dir) = p.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(json) = serde_json::to_string_pretty(&m) {
        let _ = std::fs::write(p, json);
    }
}

pub(crate) fn tuning_file(app: &AppHandle) -> Option<std::path::PathBuf> {
    app.path().app_local_data_dir().ok().map(|d| d.join("tuning.json"))
}

const BADGE_DEFAULTS: &[(&str, &str)] = &[("delta_late", "0")];

fn badge_tuning(app: &AppHandle) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = BADGE_DEFAULTS.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
    for (k, v) in load_tuning(app) {
        match out.iter_mut().find(|(have, _)| have.eq_ignore_ascii_case(&k)) {
            Some(entry) => entry.1 = v,
            None => out.push((k, v)),
        }
    }
    out
}

pub(crate) fn load_tuning(app: &AppHandle) -> Vec<(String, String)> {
    let Some(map) = tuning_file(app)
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(&s).ok())
    else {
        return Vec::new();
    };
    map.into_iter()
        .filter_map(|(k, v)| match v {
            serde_json::Value::Number(n) => Some((k, n.to_string())),
            serde_json::Value::String(s) => Some((k, s)),
            _ => None,
        })
        .collect()
}

fn tuning_stamp(app: &AppHandle) -> Option<SystemTime> {
    tuning_file(app).and_then(|p| std::fs::metadata(p).ok()).and_then(|m| m.modified().ok())
}

fn load_characters(app: &AppHandle) -> Vec<(i32, String)> {
    characters_file(app)
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str::<std::collections::HashMap<i32, String>>(&s).ok())
        .map(|m| m.into_iter().collect())
        .unwrap_or_default()
}

fn save_characters(app: &AppHandle, store: &Store) {
    let Some(p) = characters_file(app) else { return };
    if let Some(dir) = p.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(json) = serde_json::to_string_pretty(store.characters()) {
        let _ = std::fs::write(p, json);
    }
}

const BASE_URL: &str = "https://wank.wavu.wiki";

const POLL: Duration = Duration::from_secs(60);

const PAGE_LAG: i64 = 300;

const SPACING: Duration = Duration::from_secs(1);

const GAME_PROBE_TTL: Duration = Duration::from_secs(10);

use crate::process::game_running;

static SESSION_SINCE: Mutex<Option<std::collections::HashMap<String, i64>>> = Mutex::new(None);

fn session_since(tekken_id: &str) -> i64 {
    let mut guard = SESSION_SINCE.lock().unwrap_or_else(|e| e.into_inner());
    *guard.get_or_insert_with(Default::default).entry(tekken_id.to_string()).or_insert_with(now)
}

struct Running {
    stop: Arc<AtomicBool>,
    wake: Arc<AtomicBool>,
    refetch: Arc<AtomicBool>,
}

#[derive(Default)]
pub struct FeedState(Mutex<Option<Running>>);

pub fn init(app: &AppHandle) {
    app.manage(FeedState::default());
    app.manage(SessionState::default());
}

pub fn start(
    app: &AppHandle,
    tekken_id: String,
    name: String,
    fallback: String,
    own_times: Vec<i64>,
    page: Vec<Rating>,
    history: Vec<HistoryRow>,
) {
    let stop = Arc::new(AtomicBool::new(false));
    let wake = Arc::new(AtomicBool::new(false));
    let refetch = Arc::new(AtomicBool::new(false));
    if let Some(state) = app.try_state::<FeedState>() {
        if let Ok(mut cur) = state.0.lock() {
            if let Some(old) = cur.take() {
                old.stop.store(true, Ordering::SeqCst);
            }
            *cur = Some(Running { stop: stop.clone(), wake: wake.clone(), refetch: refetch.clone() });
        }
    }
    let app = app.clone();
    std::thread::spawn(move || run(app, tekken_id, name, fallback, own_times, page, history, stop, wake, refetch));
}

pub fn stop(app: &AppHandle) {
    if let Some(state) = app.try_state::<FeedState>() {
        if let Ok(mut cur) = state.0.lock() {
            if let Some(old) = cur.take() {
                old.stop.store(true, Ordering::SeqCst);
            }
        }
    }
}

pub fn wake(app: &AppHandle) {
    if let Some(state) = app.try_state::<FeedState>() {
        if let Ok(cur) = state.0.lock() {
            if let Some(r) = cur.as_ref() {
                r.wake.store(true, Ordering::SeqCst);
            }
        }
    }
}

pub fn refetch(app: &AppHandle) -> bool {
    let Some(state) = app.try_state::<FeedState>() else { return false };
    let Ok(cur) = state.0.lock() else { return false };
    let Some(r) = cur.as_ref() else { return false };
    r.refetch.store(true, Ordering::SeqCst);
    r.wake.store(true, Ordering::SeqCst);
    true
}

static CENTRAL_LOADING: AtomicBool = AtomicBool::new(false);

pub(crate) fn central_loading(app: &AppHandle, on: bool) {
    if CENTRAL_LOADING.swap(on, Ordering::SeqCst) != on {
        let _ = app.emit("feed:loading", on);
    }
}

pub(crate) fn is_central_loading() -> bool {
    CENTRAL_LOADING.load(Ordering::SeqCst)
}

static LAST_POLLED: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0);

pub(crate) fn polled(app: &AppHandle, at: i64) {
    if at <= 0 {
        return;
    }
    LAST_POLLED.store(at, Ordering::SeqCst);
    let _ = app.emit("feed:polled", at);
}

static CENTRAL_AT: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0);

pub(crate) fn central_at(app: &AppHandle, at: i64) {
    if at <= 0 || CENTRAL_AT.swap(at, Ordering::SeqCst) == at {
        return;
    }
    let _ = app.emit("feed:central", at);
}

pub(crate) fn last_central_at() -> Option<i64> {
    let t = CENTRAL_AT.load(Ordering::SeqCst);
    (t > 0).then_some(t)
}

static FEED_BEAT: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0);

#[cfg(feature = "dash")]
pub(crate) fn feed_beat() -> Option<i64> {
    let t = FEED_BEAT.load(Ordering::SeqCst);
    (t > 0).then_some(t)
}

#[cfg(feature = "dash")]
pub(crate) fn feed_running(app: &AppHandle) -> bool {
    let Some(state) = app.try_state::<FeedState>() else { return false };
    let Ok(cur) = state.0.lock() else { return false };
    cur.as_ref().is_some()
}

pub(crate) fn clear_polled() {
    LAST_POLLED.store(0, Ordering::SeqCst);
    CENTRAL_AT.store(0, Ordering::SeqCst);
    if let Ok(mut d) = DIAG_FEED.lock() {
        *d = None;
    }
}

#[derive(Serialize, Clone)]
pub struct DiagCharacter {
    pub character: String,
    pub code: Option<String>,
    pub page_mu: Option<i32>,
    pub base: i32,
    pub asof: Option<i64>,
    pub rd: Option<f64>,
}

#[derive(Serialize, Clone)]
pub struct DiagMatch {
    pub at: i64,
    pub character: Option<String>,
    pub rated: bool,
    pub change: Option<i32>,
    pub mr_after: Option<i32>,
}

#[derive(Serialize, Clone)]
pub struct DiagSlotWrite {
    pub at: i64,
    pub forced: bool,
    pub bytes: usize,
    pub replay_rows: usize,
    pub replay_rows_minutes: i64,
    pub names: usize,
    pub name_hours: f64,
    pub trimmed: bool,
    pub build_ms: u128,
    pub write_ms: u128,
}

#[derive(Serialize, Clone, Default)]
pub struct DiagFeed {
    pub characters: Vec<DiagCharacter>,
    pub recent_matches: Vec<DiagMatch>,
    pub records: usize,
    pub players: usize,
    pub rd_entries: usize,
    pub page_fetched_at: Option<i64>,
    pub last_slot_write: Option<DiagSlotWrite>,
    pub last_deferred_at: Option<i64>,
    pub last_replay_save_at: Option<i64>,
    pub central_hours_loaded: usize,
    pub central_minutes_merged: usize,
    pub last_write_error: Option<String>,
}

static DIAG_FEED: Mutex<Option<DiagFeed>> = Mutex::new(None);

fn diag_update(f: impl FnOnce(&mut DiagFeed)) {
    if let Ok(mut d) = DIAG_FEED.lock() {
        f(d.get_or_insert_with(DiagFeed::default));
    }
}

pub(crate) fn diag_feed() -> Option<DiagFeed> {
    DIAG_FEED.lock().ok().and_then(|d| d.clone())
}

#[cfg(feature = "demo")]
pub(crate) fn set_diag_feed(d: DiagFeed) {
    if let Ok(mut cur) = DIAG_FEED.lock() {
        *cur = Some(d);
    }
}

pub(crate) fn last_polled() -> Option<i64> {
    let t = LAST_POLLED.load(Ordering::SeqCst);
    (t > 0).then_some(t)
}

pub(crate) fn now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

pub(crate) const DEFAULT_FEED_URL: &str = "https://mr.tekkenresourcehub.com";

pub(crate) fn central_feed_url(app: &AppHandle) -> Option<String> {
    let configured = std::env::var("MURATING_FEED_URL")
        .ok()
        .or_else(|| tray::load_value(app, "feed_url").and_then(|v| v.as_str().map(str::to_string)))
        .map(|s| s.trim().trim_end_matches('/').to_string());
    #[cfg(debug_assertions)]
    if configured.as_deref() == Some("wavu") {
        return None;
    }
    Some(
        configured
            .filter(|s| s.starts_with("https://") || s.starts_with("http://"))
            .unwrap_or_else(|| DEFAULT_FEED_URL.to_string()),
    )
}

#[derive(Default)]
struct Central {
    newest_at: i64,
    etag: Option<String>,
    seen: std::collections::BTreeSet<i64>,
    hours_done: std::collections::BTreeSet<i64>,
    started: bool,
}

fn central_cycle(
    base: &str,
    c: &mut Central,
    store: &mut Store,
    rd: &mut murating_core::rd::RdTable,
    own_times: &[i64],
    stop: &AtomicBool,
) -> (usize, bool) {
    use crate::diag::{net_err, net_ok, Endpoint};
    use murating_core::central as cf;
    let mut requests = 1usize;
    let (answer, ms) = crate::diag::timed(|| cf::get_dated(&cf::latest_url(base), USER_AGENT, c.etag.as_deref()));
    let latest = match answer {
        Ok((cf::Fetched::NotModified, date)) => {
            net_ok(Endpoint::Feed, ms, date);
            return (requests, true);
        }
        Ok((cf::Fetched::Body { text, etag }, date)) => {
            net_ok(Endpoint::Feed, ms, date);
            match cf::parse_latest(&text) {
                Ok(l) => {
                    c.etag = etag;
                    c.newest_at = l.newest_at;
                    l
                }
                Err(e) => {
                    crate::diag!("[feed] central latest.json: {e}");
                    net_err(Endpoint::Feed, format!("latest.json did not parse: {e}"));
                    return (requests, false);
                }
            }
        }
        Ok((cf::Fetched::Missing, _)) => {
            crate::diag!("[feed] central latest.json missing");
            net_err(Endpoint::Feed, "latest.json missing (404)");
            return (requests, false);
        }
        Err(e) => {
            crate::diag!("[feed] central latest.json: {e}");
            net_err(Endpoint::Feed, format!("latest.json: {e}"));
            return (requests, false);
        }
    };
    if !c.started {
        for h in cf::hours_wanted(&latest, now(), feed::NAME_HORIZON, own_times, feed::OWN_HORIZON) {
            if stop.load(Ordering::SeqCst) {
                return (requests, false);
            }
            requests += 1;
            match cf::get_records(&cf::hour_url(base, h), USER_AGENT, |r| store.wants(r)) {
                Ok(Some(recs)) => {
                    store.merge_with_rd(&recs, rd);
                    c.hours_done.insert(h);
                }
                Ok(None) => {}
                Err(murating_core::replays::ReplayError::Body(e)) => {
                    crate::diag!("[feed] central hour file {h}: {e}");
                    net_err(Endpoint::Feed, format!("hour file {h} did not parse: {e}"));
                }
                Err(e) => {
                    crate::diag!("[feed] central hour {h}: {e}");
                    net_err(Endpoint::Feed, format!("hour file {h}: {e}"));
                    return (requests, false);
                }
            }
        }
        c.started = true;
    }
    let mut minutes = latest.minutes.clone();
    minutes.sort_unstable();
    for m in &minutes {
        if c.seen.contains(m) || c.hours_done.contains(&m.div_euclid(60)) {
            continue;
        }
        if stop.load(Ordering::SeqCst) {
            return (requests, false);
        }
        requests += 1;
        match cf::get_records(&cf::minute_url(base, *m), USER_AGENT, |r| store.wants(r)) {
            Ok(Some(recs)) => {
                store.merge_with_rd(&recs, rd);
                c.seen.insert(*m);
            }
            Ok(None) => {
                c.seen.insert(*m);
            }
            Err(murating_core::replays::ReplayError::Body(e)) => {
                crate::diag!("[feed] central minute file {m}: {e}");
                net_err(Endpoint::Feed, format!("minute file {m} did not parse: {e}"));
                c.seen.insert(*m);
            }
            Err(e) => {
                crate::diag!("[feed] central minute {m}: {e}");
                net_err(Endpoint::Feed, format!("minute file {m}: {e}"));
                return (requests, false);
            }
        }
    }
    c.seen.retain(|m| minutes.binary_search(m).is_ok());
    (requests, true)
}

fn fetch(end: i64, now: i64) -> Result<Vec<Record>, replays::ReplayError> {
    let recs = replays::fetch_window(BASE_URL, Some(end.min(now)), USER_AGENT)?;
    Ok(recs
        .into_iter()
        .filter(|r| r.battle_at > end - WINDOW_SECS && r.battle_at <= end)
        .collect())
}

struct Writer {
    tekken_id: String,
    name: String,
    power: Option<i64>,
    fallback: String,
    fallback_at: i64,
    last: Option<u64>,
    page: Vec<Rating>,
    last_ratings: Option<RatingsUpdate>,
    ledger: Ledger,
    rd: murating_core::rd::RdTable,
    game_seen: Option<(std::time::Instant, bool)>,
    history: Vec<HistoryRow>,
    since: i64,
    last_session: Option<murating_core::session::SessionSummary>,
    ach: murating_core::achievements::Progress,
    ach_session: murating_core::achievements::SessionState,
    caught_up: bool,
    catching_up: bool,
}

impl Writer {
    fn chara_of(&self, app: &AppHandle, store: &Store, code: &str) -> Option<i32> {
        crate::roster::codes(app, store.characters())
            .into_iter()
            .find(|(_, c)| c == code)
            .map(|(id, _)| id)
    }

    fn in_game(&mut self) -> bool {
        if let Some((asked, answer)) = self.game_seen {
            if asked.elapsed() < GAME_PROBE_TTL {
                return answer;
            }
        }
        let answer = game_running();
        self.game_seen = Some((std::time::Instant::now(), answer));
        answer
    }

    fn publish(&mut self, app: &AppHandle, store: &Store, stop: &AtomicBool, force: bool) {
        if stop.load(Ordering::SeqCst) {
            return;
        }
        let live = store.live_ratings(&self.page);
        let matches = store.own_matches();
        self.emit_ratings(app, &live);
        self.emit_session(app, &matches);
        let mut counted = matches.clone();
        counted.extend(murating_core::session::history_matches(&self.tekken_id, &matches, &self.history));
        crate::achievements::refresh(
            app,
            &self.tekken_id.clone(),
            &mut self.ach,
            &mut self.ach_session,
            &counted,
            &live,
            self.since,
            self.catching_up,
        );
        let payload = match store.latest_own() {
            Some((at, mr)) if at > self.fallback_at - PAGE_LAG => format!("{mr} MR"),
            _ => self.fallback.clone(),
        };

        let mut name = self.name.clone();
        if let Some((battle_name, power)) = store.latest_own_power() {
            name = battle_name;
            if self.power != Some(power) {
                self.power = Some(power);
                save_own_power(app, &self.tekken_id, power);
            }
        }
        let codes = crate::roster::codes(app, store.characters());
        let mut current: BTreeMap<String, i32> = BTreeMap::new();
        let mut asof: BTreeMap<String, i64> = BTreeMap::new();
        for l in store.live_ratings(&self.page) {
            if let Some(code) = crate::roster::code_for_name(app, &l.rating.character) {
                current.insert(code.clone(), l.rating.mu);
                if let Some(at) = l.rating.last_seen {
                    asof.insert(code, at);
                }
            }
        }
        let mut ledger_changed = false;
        for b in store.own_battles() {
            let Some(code) = codes.get(&b.chara_id) else { continue };
            if self.ledger.battles.contains(&b.battle_at) {
                continue;
            }
            let before = self.ledger.shown.get(code).copied();
            self.ledger.own_battle(b.battle_at, code, b.rounds, b.god);
            ledger_changed = true;
            crate::diag!(
                "[delta] battle {} on {code} ({} rounds, god {}): shown {:?} -> {:?}",
                b.battle_at, b.rounds, b.god, before.map(|s| s.mr), self.ledger.shown.get(code).map(|s| s.mr)
            );
        }
        if self.ledger.settle(now()) {
            crate::diag!("[delta] a replay save had no ranked battle behind it: restored {:?}", self.ledger.shown);
            ledger_changed = true;
        }
        if ledger_changed {
            save_ledger(app, &self.tekken_id, &self.ledger);
        }
        let deltas = self.ledger.deltas(&current);
        {
            let characters = live
                .iter()
                .map(|l| {
                    let code = crate::roster::code_for_name(app, &l.rating.character);
                    let rd = code
                        .as_deref()
                        .and_then(|c| self.chara_of(app, store, c))
                        .map(|chara| murating_core::glicko::rd_of(self.rd.phi(&self.tekken_id, chara)));
                    DiagCharacter {
                        page_mu: self.page.iter().find(|r| r.character == l.rating.character).map(|r| r.mu),
                        base: l.rating.mu,
                        asof: l.rating.last_seen,
                        rd,
                        character: l.rating.character.clone(),
                        code,
                    }
                })
                .collect();
            let mut recent: Vec<&murating_core::feed::OwnMatch> = matches.iter().collect();
            recent.sort_by_key(|m| m.battle_at);
            let recent_matches = recent
                .iter()
                .rev()
                .take(5)
                .rev()
                .map(|m| DiagMatch {
                    at: m.battle_at,
                    character: m.character.clone(),
                    rated: m.change.is_some(),
                    change: m.change,
                    mr_after: m.change.map(|c| m.mu_before + c),
                })
                .collect();
            let (records, players, rd_entries) = (store.len(), store.players_len(), self.rd.len());
            diag_update(|d| {
                d.characters = characters;
                d.recent_matches = recent_matches;
                d.records = records;
                d.players = players;
                d.rd_entries = rd_entries;
            });
        }
        if !force && self.in_game() {
            diag_update(|d| d.last_deferred_at = Some(now()));
            return;
        }
        let extras = feed::Extras {
            own: Some(feed::OwnName {
                name,
                power: self.power,
                text: payload.clone(),
                per_code: current.iter().map(|(c, mu)| (c.clone(), format!("{mu} MR"))).collect(),
            }),
            tuning: badge_tuning(app),
            codes,
            deltas: deltas.clone(),
            bases: current.iter().map(|(c, mu)| (c.clone(), *mu)).collect(),
            asofs: asof.iter().map(|(c, at)| (c.clone(), *at)).collect(),
            rds: self
                .page
                .iter()
                .filter_map(|r| {
                    let code = crate::roster::code_for_name(app, &r.character)?;
                    let chara = self.chara_of(app, store, &code)?;
                    Some((code, murating_core::glicko::rd_of(self.rd.phi(&self.tekken_id, chara))))
                })
                .collect(),
            opponent_phi: self.rd.median_phi(),
            rd: self.rd.clone(),
        };

        let build_start = std::time::Instant::now();
        let (bytes, rows, horizon, names, name_horizon) = if tray::mr_enabled(app) {
            let built = feed::slot_bytes(store, now(), &payload, &extras);
            (built.bytes, built.rows, built.horizon, built.names, built.name_horizon)
        } else {
            (slot::replay_bytes("", &[], &[]), 0, 0, 0, 0)
        };

        if names == 0 && !self.caught_up && tray::mr_enabled(app) {
            crate::diag!("[feed] name table still empty and the backfill has not caught up: keeping what is on disk");
            return;
        }

        let mut h = DefaultHasher::new();
        bytes.hash(&mut h);
        let digest = h.finish();
        if self.last == Some(digest) {
            return;
        }
        let build_ms = build_start.elapsed().as_millis();
        let write_start = std::time::Instant::now();
        match slot::write_slot(&paths::slot_dir(), &bytes) {
            Ok(_) => {
                let write_ms = write_start.elapsed().as_millis();
                self.last = Some(digest);
                tray::set_payload(app, &payload);
                if tray::mr_enabled(app) {
                    self.ledger.wrote(now(), &current);
                }
                if !deltas.is_empty() {
                    crate::diag!("[delta] offering {:?}", deltas);
                }
                crate::diag!(
                    "[feed] wrote {} KB in {write_ms} ms (built in {build_ms} ms): {rows} rows ({} min), {names} names ({} h), payload {payload:?}",
                    bytes.len() / 1024,
                    horizon / 60,
                    name_horizon / 3600
                );
                let write = DiagSlotWrite {
                    at: now(),
                    forced: force,
                    bytes: bytes.len(),
                    replay_rows: rows,
                    replay_rows_minutes: horizon / 60,
                    names,
                    name_hours: name_horizon as f64 / 3600.0,
                    trimmed: tray::mr_enabled(app)
                        && (horizon < feed::REPLAY_ROWS_HORIZON || name_horizon < feed::NAME_HORIZON),
                    build_ms,
                    write_ms,
                };
                diag_update(|d| {
                    d.last_slot_write = Some(write);
                    d.last_write_error = None;
                });
            }
            Err(e) => {
                crate::diag!("[feed] slot write refused: {e}");
                let e = e.to_string();
                diag_update(|d| d.last_write_error = Some(e));
            }
        }
    }
}

impl Writer {
    fn emit_ratings(&mut self, app: &AppHandle, live: &[murating_core::feed::LiveRating]) {
        let rows = char_ratings(live);
        let best = live
            .iter()
            .max_by_key(|l| (l.rating.mu, l.rating.last_seen.unwrap_or(i64::MIN)))
            .map(|l| (murating_core::characters::full_name(&l.rating.character), l.rating.mu));
        let Some((character, mu)) = best else { return };
        let update = RatingsUpdate { character, mu, ratings: rows };
        if self.last_ratings.as_ref() == Some(&update) {
            return;
        }
        if self.last_ratings.as_ref().map(|u| (&u.character, u.mu)) != Some((&update.character, update.mu)) {
            if let Some(top) = live.iter().max_by_key(|l| (l.rating.mu, l.rating.last_seen.unwrap_or(i64::MIN))) {
                crate::accounts::set_best(app, &self.tekken_id, &top.rating.character, top.rating.mu);
            }
        }
        crate::follow::update_ratings(app, &self.tekken_id, &update);
        let _ = app.emit("ratings:update", update.clone());
        self.last_ratings = Some(update);
    }

    fn emit_session(&mut self, app: &AppHandle, matches: &[murating_core::feed::OwnMatch]) {
        use murating_core::session;
        let results: Vec<session::OwnResult> = matches.iter().map(murating_core::feed::OwnMatch::to_result).collect();
        let points = session::points(&self.tekken_id, &results, &self.history);
        let fallback = player::current(&self.page).map(|r| (r.character.clone(), r.mu));
        let summary = session::summarize(&points, self.since, fallback.as_ref().map(|(c, mu)| (c.as_str(), *mu)));
        if self.last_session.as_ref() == Some(&summary) {
            return;
        }
        set_session(app, &self.tekken_id, summary.clone());
        let _ = app.emit("session:update", summary.clone());
        self.last_session = Some(summary);
    }
}

#[derive(Default)]
pub struct SessionState(Mutex<Option<(String, murating_core::session::SessionSummary)>>);

pub(crate) fn set_session(app: &AppHandle, tekken_id: &str, s: murating_core::session::SessionSummary) {
    if let Some(state) = app.try_state::<SessionState>() {
        if let Ok(mut cur) = state.0.lock() {
            *cur = Some((tekken_id.to_string(), s));
        }
    }
}

pub fn session(app: &AppHandle) -> Option<murating_core::session::SessionSummary> {
    let following = tray::current_tekken_id(app)?;
    let state = app.try_state::<SessionState>()?;
    let cur = state.0.lock().ok()?;
    cur.as_ref().filter(|(id, _)| *id == following).map(|(_, s)| s.clone())
}

#[allow(clippy::too_many_arguments)]
fn run(
    app: AppHandle,
    tekken_id: String,
    name: String,
    fallback: String,
    own_times: Vec<i64>,
    page: Vec<Rating>,
    mut history: Vec<HistoryRow>,
    stop: Arc<AtomicBool>,
    wake: Arc<AtomicBool>,
    refetch: Arc<AtomicBool>,
) {
    let mut store = Store::new(&tekken_id);
    store.add_characters(load_characters(&app));
    store.restore_players(load_players(&app), now(), &own_times);
    let mut players_saved_at = now();
    let mut tuning_seen = tuning_stamp(&app);
    let mut writer = Writer {
        power: load_own_power(&app, &tekken_id),
        tekken_id: tekken_id.clone(),
        name,
        fallback,
        fallback_at: now(),
        last: None,
        game_seen: None,
        page,
        last_ratings: None,
        ledger: load_ledger(&app, &tekken_id),
        rd: load_rd(&app),
        history: history.clone(),
        since: session_since(&tekken_id),
        last_session: None,
        ach: crate::achievements::load(&app, &tekken_id),
        ach_session: murating_core::achievements::SessionState::new(session_since(&tekken_id)),
        caught_up: false,
        catching_up: true,
    };
    {
        let page = writer.page.clone();
        seed_rd_from_page(&app, &mut writer, &page, &store);
    }
    let mut page_fetched_at = now();
    let mut own_latest_seen: Option<i64> = None;
    let mut own_arrived_at = now();
    if let Ok(mut d) = DIAG_FEED.lock() {
        *d = Some(DiagFeed { page_fetched_at: Some(page_fetched_at), ..DiagFeed::default() });
    }
    let mut last_wavu_ok: i64 = 0;
    let mut last_wavu_sent: i64 = 0;
    let mut replay_seen = newest_replay_save();
    let mut replay_checked = std::time::Instant::now();
    let mut first_cycle = true;
    let mut flush_after_fetch = false;
    let mut late_start_flush = writer.in_game();
    let central_base = central_feed_url(&app);
    let mut central = Central::default();
    if let Some(base) = &central_base {
        crate::diag!("[feed] reading the central feed at {base}");
    }

    loop {
        let t = now();
        FEED_BEAT.store(t, Ordering::SeqCst);
        let mut requests = 0usize;
        let mut wrote_live = false;
        let force_first = first_cycle && !writer.in_game();
        first_cycle = false;

        if let Some(base) = central_base.as_deref() {
            if refetch.swap(false, Ordering::SeqCst) {
                crate::diag!("[feed] reconnect: reading the central feed from scratch");
                central = Central::default();
            }
            let cold = !central.started;
            if cold {
                crate::feed::central_loading(&app, true);
            }
            let (n, ok) = central_cycle(base, &mut central, &mut store, &mut writer.rd, &own_times, &stop);
            if cold {
                crate::feed::central_loading(&app, !central.started);
            }
            if stop.load(Ordering::SeqCst) {
                return;
            }
            let (hours, minutes) = (central.hours_done.len(), central.seen.len());
            diag_update(|d| {
                d.central_hours_loaded = hours;
                d.central_minutes_merged = minutes;
            });
            requests = n;
            if ok {
                last_wavu_ok = now();
            }
            store.evict(now());
            crate::feed::central_at(&app, central.newest_at);
            if force_first {
                writer.publish(&app, &store, &stop, true);
            }
        }

        let windows = if central_base.is_some() { Vec::new() } else { store.wanted(t, &own_times) };
        for end in windows {
            if stop.load(Ordering::SeqCst) {
                return;
            }
            let live = feed::is_live(t, end);
            if !live && !wrote_live {
                store.evict(now());
                writer.publish(&app, &store, &stop, force_first);
                wrote_live = true;
            } else if !live && requests % BACKFILL_PUBLISH == 0 {
                store.evict(now());
                writer.publish(&app, &store, &stop, false);
            }
            if requests > 0 {
                std::thread::sleep(SPACING);
            }
            requests += 1;
            let (window, ms) = crate::diag::timed(|| fetch(end, t));
            match window {
                Ok(recs) => {
                    crate::diag::net_ok(crate::diag::Endpoint::Wavu, ms, None);
                    store.merge_with_rd(&recs, &mut writer.rd);
                    last_wavu_ok = now();
                    if !live {
                        store.settle(end);
                    }
                }
                Err(e) => {
                    crate::diag!("[feed] window ending {end}: {e}");
                    crate::diag::net_err(crate::diag::Endpoint::Wavu, format!("replay window: {e}"));
                    break;
                }
            }
        }

        if last_wavu_ok != last_wavu_sent {
            last_wavu_sent = last_wavu_ok;
            if !stop.load(Ordering::SeqCst) {
                polled(&app, last_wavu_ok);
            }
        }
        store.evict(now());
        let quiet = writer.in_game();
        if !quiet && requests > 0
            && (now() - players_saved_at >= PLAYERS_SAVE || store.wanted(now(), &own_times).len() <= 2)
        {
            save_players(&app, &store);
            players_saved_at = now();
        }
        if store.learn_characters(&history) > 0 {
            save_characters(&app, &store);
        }
        writer.rd.evict(now());
        if !quiet {
            save_rd(&app, &writer.rd);
        }
        let since_page = now() - page_fetched_at;
        let unknown_character = store.own_character_unknown() && since_page > PAGE_REFETCH;
        let latest_own = store.latest_own_at();
        if latest_own != own_latest_seen {
            own_latest_seen = latest_own;
            own_arrived_at = now();
        }
        let own_battle_newer = latest_own.map_or(false, |at| at > page_fetched_at)
            && (since_page >= OWN_BATTLE_REFETCH || now() - own_arrived_at >= OWN_BATTLE_QUIET);
        let new_character = crate::roster::waiting(&app) && since_page > crate::roster::ROSTER_REFETCH;
        if unknown_character || new_character || own_battle_newer {
            if own_battle_newer {
                crate::diag!("[feed] own battle newer than the page: refetching for an exact |me_base");
            }
            page_fetched_at = now();
            let (page, ms) = crate::diag::timed(|| player::fetch(BASE_URL, &tekken_id, USER_AGENT));
            match page {
                Ok(html) => {
                    crate::diag::net_ok(crate::diag::Endpoint::Wavu, ms, None);
                    last_wavu_ok = now();
                    diag_update(|d| d.page_fetched_at = Some(page_fetched_at));
                    crate::roster::observe_page(&app, &html);
                    history = player::parse_history(&html);
                    writer.history = history.clone();
                    if let Ok(fresh) = player::parse_ratings(&html) {
                        seed_rd_from_page(&app, &mut writer, &fresh, &store);
                        writer.page = fresh;
                    }
                    if store.learn_characters(&history) > 0 {
                        save_characters(&app, &store);
                    }
                }
                Err(e) => {
                    crate::diag!("[feed] player page refetch: {e}");
                    crate::diag::net_err(crate::diag::Endpoint::Wavu, format!("player page: {e}"));
                }
            }
        }
        let caught_up = if central_base.is_some() {
            central.started
        } else {
            store.wanted(now(), &own_times).len() <= 2
        };
        writer.caught_up = caught_up;
        if late_start_flush && caught_up && requests > 0 {
            crate::diag!("[feed] started mid-session: writing the caught-up table once");
            flush_after_fetch = true;
            late_start_flush = false;
        }
        writer.publish(&app, &store, &stop, flush_after_fetch);
        writer.catching_up = false;
        if flush_after_fetch {
            flush_after_fetch = false;
            save_players(&app, &store);
            players_saved_at = now();
            save_rd(&app, &writer.rd);
        }

        let mut waited = Duration::ZERO;
        while waited < POLL {
            if stop.load(Ordering::SeqCst) {
                return;
            }
            let stamp = tuning_stamp(&app);
            let retuned = stamp != tuning_seen;
            tuning_seen = stamp;
            let mut replay_saved = false;
            if replay_checked.elapsed() >= REPLAY_CHECK {
                replay_checked = std::time::Instant::now();
                let newest = newest_replay_save();
                if let (Some(t), seen) = (newest, replay_seen) {
                    if seen.map_or(true, |s| t > s) {
                        crate::diag!("[delta] replay saved at {t}");
                        diag_update(|d| d.last_replay_save_at = Some(t));
                        writer.ledger.replay_saved(t);
                        save_ledger(&app, &tekken_id, &writer.ledger);
                        replay_saved = true;
                    }
                }
                if newest.is_some() {
                    replay_seen = newest;
                }
            }
            let settle = writer.ledger.settle_due(now());
            let woke = wake.swap(false, Ordering::SeqCst);
            if woke || retuned || replay_saved || settle {
                writer.publish(&app, &store, &stop, woke || retuned || replay_saved);
            }
            if replay_saved {
                save_players(&app, &store);
                players_saved_at = now();
                save_rd(&app, &writer.rd);
                flush_after_fetch = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(500));
            waited += Duration::from_millis(500);
        }
    }
}
