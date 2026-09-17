use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, WebviewWindowBuilder};

const CARD_GAP_X: f64 = 32.0;
const CARD_GAP_Y: f64 = 56.0;

const INSET_X: f64 = 32.0;
const INSET_BOTTOM: f64 = 46.0;

static SEQ: AtomicU64 = AtomicU64::new(0);

static PENDING: Mutex<Option<Payload>> = Mutex::new(None);
static BUILDING: AtomicBool = AtomicBool::new(false);
static GENERATION: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Serialize)]
pub struct Payload {
    seq: u64,
    text: String,
    emphasis: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rarity: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
}

pub fn achievement(app: &AppHandle, e: &murating_core::achievements::Entry) {
    let kind = match e.kind {
        murating_core::achievements::Kind::Milestone => "milestone",
        murating_core::achievements::Kind::Achievement => "achievement",
    };
    show_card(
        app,
        &e.text,
        e.emphasis.clone(),
        Some(kind.to_string()),
        Some(e.rarity),
        Some(e.title.clone()),
    );
}

pub fn established(app: &AppHandle, name: &str) {
    show(app, "Mu Rating HUD established for", Some(name.to_string()));
}

pub fn switched(app: &AppHandle, name: &str) {
    show(app, "Account switch detected, now following", Some(name.to_string()));
}

pub fn new_account(app: &AppHandle) {
    show(app, "New Steam account detected, add its", Some("Tekken ID".into()));
}

pub fn minimized(app: &AppHandle) {
    show(app, "Mu Rating HUD still running in the", Some("tray".into()));
}

fn show(app: &AppHandle, text: &str, emphasis: Option<String>) {
    show_card(app, text, emphasis, None, None, None);
}

fn show_card(
    app: &AppHandle,
    text: &str,
    emphasis: Option<String>,
    kind: Option<String>,
    rarity: Option<u8>,
    title: Option<String>,
) {
    if crate::tray::notifications_hidden(app) || crate::tray::headless(app) {
        return;
    }
    deliver(app, Payload {
        seq: SEQ.fetch_add(1, Ordering::Relaxed),
        text: text.to_string(),
        emphasis,
        kind,
        rarity,
        title,
    });
}

fn deliver(app: &AppHandle, payload: Payload) {
    if let Some(win) = app.get_webview_window("toast") {
        let _ = win.emit("toast:show", payload);
        present(app, &win);
        return;
    }
    if let Ok(mut p) = PENDING.lock() {
        *p = Some(payload);
    }
    if BUILDING.swap(true, Ordering::SeqCst) {
        return;
    }
    let app = app.clone();
    std::thread::spawn(move || {
        let built = app
            .config()
            .app
            .windows
            .iter()
            .find(|w| w.label == "toast")
            .cloned()
            .map(|cfg| WebviewWindowBuilder::from_config(&app, &cfg).and_then(|b| b.build()));
        BUILDING.store(false, Ordering::SeqCst);
        if let Some(Err(e)) = built {
            crate::diag!("[toast] could not build the toast window: {e}");
        }
    });
}

#[tauri::command]
pub fn toast_ready(app: AppHandle) -> Option<Payload> {
    let payload = PENDING.lock().ok().and_then(|mut p| p.take())?;
    if let Some(win) = app.get_webview_window("toast") {
        present(&app, &win);
    }
    Some(payload)
}

pub fn on_destroyed() {
    GENERATION.fetch_add(1, Ordering::SeqCst);
}

fn present(app: &AppHandle, win: &tauri::WebviewWindow) {
    let anchor = app.get_webview_window("main").unwrap_or_else(|| win.clone());
    if let Ok(Some(mon)) = anchor.current_monitor() {
        if let Ok(size) = win.outer_size() {
            let scale = mon.scale_factor();
            let m = mon.position();
            let s = mon.size();
            let x = m.x + s.width as i32
                - size.width as i32
                - ((CARD_GAP_X - INSET_X) * scale) as i32;
            let y = m.y + s.height as i32
                - size.height as i32
                - ((CARD_GAP_Y - INSET_BOTTOM) * scale) as i32;
            let _ = win.set_position(PhysicalPosition::new(x, y));
        }
    }

    let _ = win.show();
    let _ = win.set_always_on_top(true);
    if crate::tray::over_game(app) && crate::process::game_has_focus() {
        keep_on_top(win);
    }
}

fn keep_on_top(win: &tauri::WebviewWindow) {
    const HWND_TOPMOST: isize = -1;
    const SWP_NOSIZE: u32 = 0x0001;
    const SWP_NOMOVE: u32 = 0x0002;
    const SWP_NOACTIVATE: u32 = 0x0010;
    const SWP_SHOWWINDOW: u32 = 0x0040;

    #[link(name = "user32")]
    extern "system" {
        fn SetWindowPos(hwnd: isize, after: isize, x: i32, y: i32, cx: i32, cy: i32, flags: u32) -> i32;
    }

    let Ok(hwnd) = win.hwnd() else { return };
    let hwnd = hwnd.0 as isize;
    let generation = GENERATION.load(Ordering::SeqCst);
    std::thread::spawn(move || {
        for _ in 0..(TOPMOST_HOLD_MS / TOPMOST_STEP_MS) {
            if GENERATION.load(Ordering::SeqCst) != generation {
                return;
            }
            unsafe {
                SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOSIZE | SWP_NOMOVE | SWP_NOACTIVATE | SWP_SHOWWINDOW);
            }
            std::thread::sleep(std::time::Duration::from_millis(TOPMOST_STEP_MS));
        }
    });
}

const TOPMOST_HOLD_MS: u64 = 8000;
const TOPMOST_STEP_MS: u64 = 400;

pub const TEST_DELAY_SECS: u64 = 5;

const TEST_DECK: [&str; 10] = [
    "A11",
    "M06",
    "A01",
    "A13",
    "A06",
    "A04",
    "A05",
    "M01",
    "A56",
    "A16",
];

static TEST_NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

pub fn test(app: &AppHandle) {
    let id = TEST_DECK[TEST_NEXT.fetch_add(1, Ordering::SeqCst) % TEST_DECK.len()];
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(TEST_DELAY_SECS));
        let Some(item) = murating_core::achievements::item(id) else { return };
        achievement(
            &app,
            &murating_core::achievements::Entry {
                id: item.id.to_string(),
                kind: item.kind,
                rarity: item.rarity,
                title: item.title.to_string(),
                text: item.condition.to_string(),
                emphasis: None,
                at: crate::feed::now(),
            },
        );
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use murating_core::achievements::item;

    #[test]
    fn the_test_deck_shows_every_rarity_in_five_presses() {
        let rarities: Vec<u8> = TEST_DECK
            .iter()
            .map(|id| {
                let it = item(id).unwrap_or_else(|| panic!("{id} is not in the catalog"));
                assert!(it.tiers.is_empty(), "{id} is tiered, so its catalog rarity is only its top step");
                it.rarity
            })
            .collect();
        for r in 1..=5u8 {
            assert_eq!(rarities.iter().filter(|&&x| x == r).count(), 2, "rarity {r}");
        }
        for start in 0..TEST_DECK.len() {
            let mut seen: Vec<u8> = (0..5).map(|i| rarities[(start + i) % TEST_DECK.len()]).collect();
            seen.sort_unstable();
            assert_eq!(seen, vec![1, 2, 3, 4, 5], "five presses from press {start}");
        }
    }
}
