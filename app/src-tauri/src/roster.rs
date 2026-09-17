use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use murating_core::roster::{self, Roster};
use murating_core::{game, player};

const BUILD_CHECK: Duration = Duration::from_secs(300);

pub const ROSTER_REFETCH: i64 = 6 * 60 * 60;

#[derive(Default)]
pub struct RosterState(Mutex<Roster>);

fn file(app: &AppHandle) -> Option<PathBuf> {
    app.path().app_local_data_dir().ok().map(|d| d.join("roster.json"))
}

fn load(app: &AppHandle) -> Roster {
    file(app)
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save(app: &AppHandle, r: &Roster) {
    let Some(p) = file(app) else { return };
    if let Some(dir) = p.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let Ok(json) = serde_json::to_string_pretty(r) else { return };
    let tmp = p.with_extension("json.tmp");
    if std::fs::write(&tmp, json).is_ok() {
        let _ = std::fs::rename(&tmp, &p);
    }
}

pub fn init(app: &AppHandle) {
    app.manage(RosterState(Mutex::new(load(app))));
    let app = app.clone();
    std::thread::spawn(move || loop {
        check_build(&app);
        std::thread::sleep(BUILD_CHECK);
    });
}

fn update(app: &AppHandle, change: impl FnOnce(&mut Roster) -> bool) {
    let Some(state) = app.try_state::<RosterState>() else { return };
    let Ok(mut r) = state.0.lock() else { return };
    let mut changed = change(&mut r);
    if let Some(m) = r.pair() {
        crate::diag!("[roster] new character matched: {} is {} ({})", m.code, m.wavu.name, m.wavu.slug);
        changed = true;
    }
    if changed {
        save(app, &r);
        let _ = app.emit("roster:changed", status_of(&r));
    }
}

fn check_build(app: &AppHandle) {
    let build = game::build_id();
    let needs = app
        .try_state::<RosterState>()
        .and_then(|s| s.0.lock().ok().map(|r| r.needs_scan(build)))
        .unwrap_or(false);
    if !needs {
        return;
    }
    let Some(dir) = game::install_dir() else { return };
    match roster::scan_game(&dir) {
        Ok(scan) => update(app, |r| {
            let added = r.observe_game(build, &scan);
            if !added.is_empty() {
                crate::diag!("[roster] build {build:?} adds name art: {added:?}");
            }
            true
        }),
        Err(e) => crate::diag!("[roster] scan of build {build:?} failed: {e}"),
    }
}

pub fn observe_page(app: &AppHandle, html: &str) {
    let wavu = player::parse_roster(html);
    update(app, |r| r.observe_wavu(&wavu));
}

pub fn codes(app: &AppHandle, learned: &std::collections::HashMap<i32, String>) -> std::collections::HashMap<i32, String> {
    let Some(state) = app.try_state::<RosterState>() else { return Default::default() };
    let Ok(r) = state.0.lock() else { return Default::default() };
    let ids = roster::CHARA_IDS.iter().map(|(id, _)| *id).chain(learned.keys().copied());
    ids.filter_map(|id| r.code_for_chara(id, learned).map(|c| (id, c))).collect()
}

pub fn code_for_name(app: &AppHandle, name: &str) -> Option<String> {
    let state = app.try_state::<RosterState>()?;
    let r = state.0.lock().ok()?;
    r.code(name)
}

pub fn full_roster_len(app: &AppHandle) -> usize {
    app.try_state::<RosterState>()
        .and_then(|s| s.0.lock().ok().map(|r| r.full_roster().len()))
        .unwrap_or(0)
}

pub fn waiting(app: &AppHandle) -> bool {
    app.try_state::<RosterState>()
        .and_then(|s| s.0.lock().ok().map(|r| !r.waiting_codes().is_empty()))
        .unwrap_or(false)
}

#[derive(Serialize, Clone)]
pub struct RosterStatus {
    pub build: Option<u64>,
    pub waiting_codes: Vec<String>,
    pub waiting_names: Vec<String>,
    pub learned: Vec<LearnedCode>,
}

#[derive(Serialize, Clone)]
pub struct LearnedCode {
    pub code: String,
    pub playable: bool,
    pub wavu: Option<String>,
}

fn status_of(r: &Roster) -> RosterStatus {
    RosterStatus {
        build: r.build,
        waiting_codes: r.waiting_codes(),
        waiting_names: r.waiting_names().into_iter().map(|w| w.name).collect(),
        learned: r
            .learned
            .iter()
            .map(|(code, l)| LearnedCode {
                code: code.clone(),
                playable: l.playable,
                wavu: l.wavu.as_ref().map(|w| w.name.clone()),
            })
            .collect(),
    }
}

#[tauri::command]
pub fn roster_status(app: AppHandle) -> Option<RosterStatus> {
    let state = app.try_state::<RosterState>()?;
    let r = state.0.lock().ok()?;
    Some(status_of(&r))
}
