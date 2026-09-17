use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use murating_core::follow::{self as rules, Inputs, Paired, Want};
use murating_core::{paths, slot, steam};

use crate::commands::{self, Announce, Connected};
use crate::feed::RatingsUpdate;
use crate::{accounts, feed, launch, process, toast, tray};

#[derive(Serialize, Clone, PartialEq, Debug)]
#[serde(tag = "at", rename_all = "snake_case")]
pub enum FollowState {
    Setup,
    Manual,
    Paused,
    Waiting { expected: Option<String> },
    Unpaired,
    Connecting { tekken_id: String, name: String },
    Following { me: Connected, manual: bool },
    Failed { tekken_id: String, name: String, error: String, retry_at: Option<i64> },
}

static STATE: Mutex<Option<FollowState>> = Mutex::new(None);
static PIN: Mutex<Option<(String, Option<String>)>> = Mutex::new(None);
static POKE: AtomicBool = AtomicBool::new(false);
static FORCE: AtomicBool = AtomicBool::new(false);
static PAUSED: AtomicBool = AtomicBool::new(false);

const STEAM_POLL: Duration = Duration::from_secs(4);
const GAME_POLL_WAITING: Duration = Duration::from_secs(5);
const GAME_POLL_FOLLOWING: Duration = Duration::from_secs(15);
const BACKOFF: [i64; 3] = [60, 120, 300];

pub fn known_state() -> Option<FollowState> {
    STATE.lock().ok().and_then(|s| s.clone())
}

pub fn state() -> FollowState {
    STATE.lock().ok().and_then(|s| s.clone()).unwrap_or(FollowState::Setup)
}

pub(crate) fn set_state(app: &AppHandle, next: FollowState) {
    let changed = {
        let Ok(mut s) = STATE.lock() else { return };
        if s.as_ref() == Some(&next) {
            false
        } else {
            *s = Some(next.clone());
            true
        }
    };
    if changed {
        let _ = app.emit("follow:state", next);
    }
}

pub fn set_following(app: &AppHandle, me: Connected) {
    PAUSED.store(false, Ordering::SeqCst);
    let manual = PIN.lock().ok().and_then(|p| p.as_ref().map(|(id, _)| *id == me.tekken_id)).unwrap_or(false);
    set_state(app, FollowState::Following { me, manual });
}

pub fn update_ratings(app: &AppHandle, tekken_id: &str, u: &RatingsUpdate) {
    if let FollowState::Following { mut me, manual } = state() {
        if me.tekken_id == tekken_id {
            me.character = u.character.clone();
            me.mu = u.mu;
            me.ratings = u.ratings.clone();
            if let Ok(mut s) = STATE.lock() {
                *s = Some(FollowState::Following { me, manual });
            }
        }
    }
    let _ = app;
}

pub fn pin(tekken_id: &str) {
    if let Ok(mut p) = PIN.lock() {
        *p = Some((tekken_id.to_string(), steam::active_steam_id()));
    }
}

pub fn poke(_app: &AppHandle) {
    POKE.store(true, Ordering::SeqCst);
}

pub fn connect_now() {
    FORCE.store(true, Ordering::SeqCst);
    POKE.store(true, Ordering::SeqCst);
}

fn name_of(accounts: &[accounts::Account], id: &str) -> String {
    accounts.iter().find(|a| a.tekken_id == id).map(|a| a.name.clone()).unwrap_or_else(|| id.to_string())
}

fn blank(app: &AppHandle) {
    feed::stop(app);
    let _ = slot::write_slot(&paths::slot_dir(), &slot::replay_bytes("", &[], &[]));
    tray::clear_account(app);
}

pub fn pause(app: &AppHandle) {
    PAUSED.store(true, Ordering::SeqCst);
    feed::stop(app);
    tray::clear_account(app);
    feed::clear_polled();
    set_state(app, FollowState::Paused);
}

pub fn resume(app: &AppHandle) {
    PAUSED.store(false, Ordering::SeqCst);
    #[cfg(feature = "demo")]
    if true {
        crate::demo::resume(app);
        return;
    }
    if tray::auto_connect(app) {
        connect_now();
        return;
    }
    let app = app.clone();
    std::thread::spawn(move || {
        let saved = accounts::load(&app);
        let Some(id) = accounts::last_followed(&saved).or_else(|| saved.first().map(|a| a.tekken_id.clone())) else {
            set_state(&app, FollowState::Setup);
            return;
        };
        let name = name_of(&saved, &id);
        set_state(&app, FollowState::Connecting { tekken_id: id.clone(), name: name.clone() });
        if let Err(error) = commands::activate(&app, &id, None, Announce::Quiet) {
            crate::diag!("[follow] reconnect {id}: {error}");
            set_state(&app, FollowState::Failed { tekken_id: id, name, error, retry_at: None });
        }
    });
}

pub fn disconnect(app: &AppHandle) {
    if let Ok(mut p) = PIN.lock() {
        *p = None;
    }
    #[cfg(feature = "demo")]
    if true {
        crate::demo::disconnect(app);
        return;
    }
    blank(app);
    feed::clear_polled();
    accounts::save(app, &[]);
    set_state(app, FollowState::Setup);
}

pub fn start(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || run(app));
}

struct Loop {
    last_steam: Option<String>,
    game: bool,
    game_at: Option<Instant>,
    unpaired_for: Option<Option<String>>,
    failures: usize,
    retry_at: Option<i64>,
    autostart_once: bool,
    open_pending: bool,
}

fn run(app: AppHandle) {
    let mut l = Loop {
        last_steam: steam::active_steam_id(),
        game: false,
        game_at: None,
        unpaired_for: None,
        failures: 0,
        retry_at: None,
        autostart_once: launch::is_autostart(),
        open_pending: true,
    };
    let mut first = true;
    loop {
        if !first {
            let mut waited = Duration::ZERO;
            while waited < STEAM_POLL && !POKE.swap(false, Ordering::SeqCst) {
                std::thread::sleep(Duration::from_millis(250));
                waited += Duration::from_millis(250);
            }
        }
        first = false;
        tick(&app, &mut l);
    }
}

fn tick(app: &AppHandle, l: &mut Loop) {
    let steam_now = steam::active_steam_id();
    let steam_changed = steam_now != l.last_steam;
    if steam_changed {
        l.last_steam = steam_now.clone();
        l.failures = 0;
        l.retry_at = None;
        if let Ok(mut p) = PIN.lock() {
            if p.as_ref().is_some_and(|(_, s)| *s != steam_now) {
                *p = None;
            }
        }
    }
    if PAUSED.load(Ordering::SeqCst) {
        set_state(app, FollowState::Paused);
        return;
    }
    let saved = accounts::load(app);
    let following = tray::current_tekken_id(app);

    let auto = tray::auto_connect(app);
    let force = auto && FORCE.swap(false, Ordering::SeqCst);
    if force {
        l.retry_at = None;
    }
    let interval = if following.is_some() { GAME_POLL_FOLLOWING } else { GAME_POLL_WAITING };
    if force || steam_changed || l.game_at.map_or(true, |t| t.elapsed() >= interval) {
        let was = l.game;
        l.game = process::game_running();
        l.game_at = Some(Instant::now());
        if l.game && !was {
            l.failures = 0;
            l.retry_at = None;
            if !tray::headless(app) && app.get_webview_window("main").is_none() {
                crate::window::open_main(app, Some("app:show-ratings"));
            }
        }
    }

    if !auto {
        manual_tick(app, l, &saved, following, steam_changed.then_some(steam_now).flatten());
        return;
    }

    let paired: Vec<Paired> = saved
        .iter()
        .map(|a| Paired { tekken_id: a.tekken_id.clone(), steam_id: a.steam_id.clone() })
        .collect();
    if following.is_some() {
        l.open_pending = false;
    }
    let open = l.open_pending && !saved.is_empty();
    let signed_in = if open && !force && !l.game && !paired.iter().any(|a| a.steam_id.is_some() && a.steam_id == steam_now) {
        None
    } else {
        steam_now.clone()
    };
    let pinned = PIN.lock().ok().and_then(|p| p.as_ref().map(|(id, _)| id.clone()));
    let last = accounts::last_followed(&saved);
    let want = rules::want(&Inputs {
        accounts: &paired,
        game_running: l.game || force || open,
        signed_in: signed_in.as_deref(),
        following: following.as_deref(),
        pinned: pinned.as_deref(),
        last_followed: last.as_deref(),
    });

    if !matches!(want, Want::Unpaired) {
        l.unpaired_for = None;
    }
    match want {
        Want::Setup => set_state(app, FollowState::Setup),
        Want::Waiting { expected } => {
            set_state(app, FollowState::Waiting { expected: expected.map(|id| name_of(&saved, &id)) });
        }
        Want::Unpaired => {
            if l.unpaired_for.as_ref() != Some(&steam_now) {
                l.unpaired_for = Some(steam_now.clone());
                blank(app);
                toast::new_account(app);
            }
            set_state(app, FollowState::Unpaired);
        }
        Want::Follow(id) => {
            if following.as_deref() == Some(id.as_str()) {
                if !matches!(state(), FollowState::Following { .. }) {
                    l.failures = 0;
                }
                return;
            }
            if l.retry_at.is_some_and(|t| feed::now() < t) {
                return;
            }
            let name = name_of(&saved, &id);
            let announce = if pinned.as_deref() == Some(id.as_str()) {
                Announce::Quiet
            } else if following.is_some() {
                Announce::Switched
            } else {
                Announce::Established
            };
            set_state(app, FollowState::Connecting { tekken_id: id.clone(), name: name.clone() });
            match commands::activate(app, &id, None, announce) {
                Ok(_) => {
                    l.failures = 0;
                    l.retry_at = None;
                    l.open_pending = false;
                }
                Err(error) => {
                    crate::diag!("[follow] connect {id}: {error}");
                    if following.is_some() {
                        blank(app);
                    }
                    let wait = BACKOFF[l.failures.min(BACKOFF.len() - 1)];
                    l.failures += 1;
                    let retry_at = feed::now() + wait;
                    l.retry_at = Some(retry_at);
                    set_state(app, FollowState::Failed { tekken_id: id, name, error, retry_at: Some(retry_at) });
                }
            }
        }
    }
}

fn manual_tick(app: &AppHandle, l: &mut Loop, saved: &[accounts::Account], following: Option<String>, switched_to: Option<String>) {
    if saved.is_empty() {
        set_state(app, FollowState::Setup);
        return;
    }
    if following.is_none() {
        if std::mem::take(&mut l.autostart_once) {
            let signed_in = steam::active_steam_id();
            let id = saved
                .iter()
                .find(|a| a.steam_id.is_some() && a.steam_id == signed_in)
                .map(|a| a.tekken_id.clone())
                .or_else(|| accounts::last_followed(saved))
                .unwrap_or_else(|| saved[0].tekken_id.clone());
            if let Err(e) = commands::activate(app, &id, None, Announce::Established) {
                crate::diag!("[follow] autostart connect {id}: {e}");
            }
            return;
        }
        if !matches!(state(), FollowState::Connecting { .. }) {
            set_state(app, FollowState::Manual);
        }
        return;
    }
    l.autostart_once = false;
    let Some(steam_id) = switched_to else { return };
    match saved.iter().find(|a| a.steam_id.as_deref() == Some(steam_id.as_str())) {
        Some(a) if following.as_deref() == Some(a.tekken_id.as_str()) => {}
        Some(a) => {
            if let Err(e) = commands::activate(app, &a.tekken_id, None, Announce::Switched) {
                crate::diag!("[follow] switch to {}: {e}", a.tekken_id);
            }
        }
        None => {
            blank(app);
            toast::new_account(app);
            set_state(app, FollowState::Manual);
            if !tray::headless(app) {
                crate::window::open_main(app, Some("app:add-account"));
            }
        }
    }
}
