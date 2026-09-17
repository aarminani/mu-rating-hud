use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use murating_core::achievements::{self as ach, Progress, SessionState};
use murating_core::feed::{LiveRating, OwnMatch};

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Snapshot {
    pub tekken_id: String,
    pub feed: Vec<ach::FeedItem>,
    pub rows: Vec<ach::Row>,
    pub sections: Vec<ach::Sections>,
    pub earned: usize,
    pub total: usize,
    pub unread: usize,
    pub unread_rarity: u8,
}

#[derive(Default)]
pub struct AchState(Mutex<Option<Snapshot>>);

static LOCK: Mutex<()> = Mutex::new(());

pub fn init(app: &AppHandle) {
    app.manage(AchState::default());
}

fn file(app: &AppHandle, tekken_id: &str) -> Option<std::path::PathBuf> {
    app.path()
        .app_local_data_dir()
        .ok()
        .map(|d| d.join(format!("achievements_{tekken_id}.json")))
}

pub fn load(app: &AppHandle, tekken_id: &str) -> Progress {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    file(app, tekken_id)
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save(app: &AppHandle, tekken_id: &str, p: &Progress) {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let Some(path) = file(app, tekken_id) else { return };
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(json) = serde_json::to_string(p) {
        let tmp = path.with_extension("json.tmp");
        if std::fs::write(&tmp, json).is_ok() {
            let _ = std::fs::rename(&tmp, &path);
        }
    }
}

pub fn utc_offset(app: &AppHandle) -> i32 {
    crate::tray::load_value(app, "utc_offset_min")
        .and_then(|v| v.as_i64())
        .map(|v| v.clamp(-14 * 60, 14 * 60) as i32)
        .unwrap_or(0)
}

pub fn set_utc_offset(app: &AppHandle, minutes: i32) {
    let minutes = minutes.clamp(-14 * 60, 14 * 60);
    if utc_offset(app) != minutes {
        crate::tray::save_values(app, &[("utc_offset_min", minutes.into())]);
    }
}

pub(crate) fn build(tekken_id: &str, p: &Progress, s: &SessionState, i: &ach::Inputs) -> Snapshot {
    let (rows, sections) = ach::collection(p, s, i);
    let unread: Vec<&ach::FeedItem> = p
        .feed
        .iter()
        .filter(|f| matches!(f, ach::FeedItem::Award { .. }) && f.at() > p.seen_at)
        .collect();
    let unread_rarity = unread
        .iter()
        .filter_map(|f| match f {
            ach::FeedItem::Award { rarity, .. } => Some(*rarity),
            _ => None,
        })
        .max()
        .unwrap_or(0);
    Snapshot {
        tekken_id: tekken_id.to_string(),
        feed: p.feed.clone(),
        earned: rows.iter().filter(|r| r.earned).count(),
        total: rows.len(),
        rows,
        sections,
        unread: unread.len(),
        unread_rarity,
    }
}

pub(crate) fn put(app: &AppHandle, snap: Snapshot) {
    if let Some(state) = app.try_state::<AchState>() {
        if let Ok(mut cur) = state.0.lock() {
            *cur = Some(snap);
        }
    }
}

pub fn refresh(
    app: &AppHandle,
    tekken_id: &str,
    p: &mut Progress,
    s: &mut SessionState,
    matches: &[OwnMatch],
    ratings: &[LiveRating],
    session_since: i64,
    catching_up: bool,
) {
    let inputs = ach::Inputs {
        matches,
        ratings,
        roster_total: crate::roster::full_roster_len(app),
        session_since,
        now: crate::feed::now(),
        utc_offset_min: utc_offset(app),
    };
    let out = ach::update(p, s, &inputs);
    if let Some(t) = seen_mark(tekken_id) {
        p.seen_at = p.seen_at.max(t);
    }
    if out.changed {
        save(app, tekken_id, p);
    }
    let snap = build(tekken_id, p, s, &inputs);
    let had = app
        .try_state::<AchState>()
        .and_then(|st| st.0.lock().ok().and_then(|c| c.clone()));
    let moved = had.as_ref() != Some(&snap);
    put(app, snap.clone());
    if !out.entries.is_empty() {
        for e in &out.entries {
            crate::diag!("[achievements] {} {} - {}", e.id, e.title, e.text);
        }
        let _ = app.emit("achievements:new", snap);
        if let Some(toast) = out.toast {
            if catching_up {
                crate::diag!("[achievements] catching up, no toast for {}", toast.id);
            } else {
                crate::toast::achievement(app, &toast);
            }
        }
    } else if moved {
        let _ = app.emit("achievements:changed", ());
    }
}

static SEEN: Mutex<Option<std::collections::HashMap<String, i64>>> = Mutex::new(None);

fn seen_mark(tekken_id: &str) -> Option<i64> {
    SEEN.lock().ok()?.as_ref()?.get(tekken_id).copied()
}

pub fn snapshot(app: &AppHandle) -> Option<Snapshot> {
    let following = crate::tray::current_tekken_id(app)?;
    let state = app.try_state::<AchState>()?;
    let cur = state.0.lock().ok()?;
    cur.as_ref().filter(|s| s.tekken_id == following).cloned()
}

pub fn mark_seen(app: &AppHandle) -> Option<Snapshot> {
    let following = crate::tray::current_tekken_id(app)?;
    let at = crate::feed::now();
    #[cfg(not(feature = "demo"))]
    {
        let mut p = load(app, &following);
        p.seen_at = at;
        save(app, &following, &p);
    }
    if let Ok(mut seen) = SEEN.lock() {
        seen.get_or_insert_with(Default::default).insert(following.clone(), at);
    }
    let state = app.try_state::<AchState>()?;
    let mut cur = state.0.lock().ok()?;
    let snap = cur.as_mut().filter(|s| s.tekken_id == following)?;
    snap.unread = 0;
    snap.unread_rarity = 0;
    Some(snap.clone())
}
