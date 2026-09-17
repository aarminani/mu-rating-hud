use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, LogicalSize, Manager, WebviewWindow, WebviewWindowBuilder};

use crate::tray;

static INTENT: Mutex<Option<String>> = Mutex::new(None);
static EVER_READY: AtomicBool = AtomicBool::new(false);
static BUILDING: AtomicBool = AtomicBool::new(false);

pub fn open_main(app: &AppHandle, intent: Option<&str>) {
    if tray::headless(app) {
        tray::set_headless(app, false);
    }
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
        if let Some(i) = intent {
            let _ = app.emit(i, ());
        }
        let _ = app.emit("app:unparked", ());
        return;
    }
    if let Some(i) = intent {
        if let Ok(mut slot) = INTENT.lock() {
            *slot = Some(i.to_string());
        }
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
            .find(|w| w.label == "main")
            .cloned()
            .map(|cfg| WebviewWindowBuilder::from_config(&app, &cfg).and_then(|b| b.build()));
        BUILDING.store(false, Ordering::SeqCst);
        match built {
            Some(Ok(w)) => {
                std::thread::sleep(Duration::from_millis(2500));
                if !w.is_visible().unwrap_or(true) {
                    let _ = w.show();
                    let _ = w.set_focus();
                }
            }
            Some(Err(e)) => crate::diag!("[window] could not build the main window: {e}"),
            None => crate::diag!("[window] no main window in tauri.conf.json"),
        }
    });
}

#[derive(Serialize)]
pub struct WindowIntent {
    reopened: bool,
    intent: Option<String>,
}

#[tauri::command]
pub fn main_window_ready(app: AppHandle) -> WindowIntent {
    #[cfg(feature = "demo")]
    crate::demo::window_opened(&app);
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
    WindowIntent {
        reopened: EVER_READY.swap(true, Ordering::SeqCst),
        intent: INTENT.lock().ok().and_then(|mut i| i.take()),
    }
}

pub fn on_destroyed() {
    CONNECTED.store(false, Ordering::SeqCst);
}

pub const CONNECTED_MIN: (f64, f64) = (560.0, 480.0);
pub const CONNECTED_MAX: (f64, f64) = (688.0, 716.0);
const FIGHTER: (f64, f64) = (572.0, 400.0);
const SMALL_WIDTH: f64 = 440.0;

static CONNECTED: AtomicBool = AtomicBool::new(false);
static RESIZE_GEN: AtomicU64 = AtomicU64::new(0);

pub fn connected_size(saved: Option<(f64, f64)>) -> (f64, f64) {
    match saved {
        Some((w, h)) if w.is_finite() && h.is_finite() => (
            w.clamp(CONNECTED_MIN.0, CONNECTED_MAX.0),
            h.clamp(CONNECTED_MIN.1, CONNECTED_MAX.1),
        ),
        _ => CONNECTED_MIN,
    }
}

fn saved_size(app: &AppHandle) -> Option<(f64, f64)> {
    let v = tray::load_value(app, "window_size")?;
    Some((v.get("width")?.as_f64()?, v.get("height")?.as_f64()?))
}

fn frame_insets(w: &WebviewWindow) -> (f64, f64) {
    let (Ok(outer), Ok(inner), Ok(scale)) = (w.outer_size(), w.inner_size(), w.scale_factor()) else {
        return (0.0, 0.0);
    };
    let dw = f64::from(outer.width.saturating_sub(inner.width)) / scale;
    let dh = f64::from(outer.height.saturating_sub(inner.height)) / scale;
    (dw, dh)
}

fn apply(
    w: &WebviewWindow,
    resizable: bool,
    limits: Option<((f64, f64), (f64, f64))>,
    size: (f64, f64),
    center: bool,
) {
    let _ = w.set_resizable(resizable);
    let (dw, dh) = if limits.is_some() { frame_insets(w) } else { (0.0, 0.0) };
    let grown = |(lw, lh): (f64, f64)| LogicalSize::new(lw + dw, lh + dh);
    let _ = w.set_min_size(limits.map(|(min, _)| grown(min)));
    let _ = w.set_max_size(limits.map(|(_, max)| grown(max)));
    let _ = w.set_size(LogicalSize::new(size.0, size.1));
    if center {
        let _ = w.center();
    }
}

#[tauri::command]
pub fn set_window_mode(app: AppHandle, mode: String, height: Option<f64>) {
    let Some(w) = app.get_webview_window("main") else { return };
    let was_connected = CONNECTED.load(Ordering::SeqCst);
    match mode.as_str() {
        "connected" => {
            if was_connected {
                return;
            }
            let size = connected_size(saved_size(&app));
            apply(&w, true, Some((CONNECTED_MIN, CONNECTED_MAX)), size, true);
            CONNECTED.store(true, Ordering::SeqCst);
        }
        "fighter" => {
            CONNECTED.store(false, Ordering::SeqCst);
            apply(&w, false, None, FIGHTER, was_connected);
        }
        _ => {
            CONNECTED.store(false, Ordering::SeqCst);
            let h = height.unwrap_or(278.0).clamp(200.0, 720.0).ceil();
            apply(&w, false, None, (SMALL_WIDTH, h), was_connected);
        }
    }
}

pub fn on_resized(app: &AppHandle) {
    if !CONNECTED.load(Ordering::SeqCst) {
        return;
    }
    let gen = RESIZE_GEN.fetch_add(1, Ordering::SeqCst) + 1;
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(600));
        if RESIZE_GEN.load(Ordering::SeqCst) != gen || !CONNECTED.load(Ordering::SeqCst) {
            return;
        }
        let Some(w) = app.get_webview_window("main") else { return };
        if w.is_minimized().unwrap_or(false) || w.is_maximized().unwrap_or(false) {
            return;
        }
        let (Ok(size), Ok(scale)) = (w.inner_size(), w.scale_factor()) else { return };
        let logical = size.to_logical::<f64>(scale);
        let (width, height) = connected_size(Some((logical.width, logical.height)));
        tray::save_values(&app, &[("window_size", serde_json::json!({ "width": width, "height": height }))]);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connected_size_stays_between_the_limits() {
        assert_eq!(connected_size(None), (560.0, 480.0));
        assert_eq!(connected_size(Some((400.0, 900.0))), (560.0, 716.0));
        assert_eq!(connected_size(Some((800.0, 600.0))), (688.0, 600.0));
        assert_eq!(connected_size(Some((640.0, 520.0))), (640.0, 520.0));
        assert_eq!(connected_size(Some((f64::NAN, 600.0))), (560.0, 480.0));
    }

    #[test]
    fn the_limits_are_ordered() {
        assert!(CONNECTED_MIN.0 <= CONNECTED_MAX.0 && CONNECTED_MIN.1 <= CONNECTED_MAX.1);
        assert!(FIGHTER.0 >= CONNECTED_MIN.0, "four fighter cards need the connected width");
    }
}
