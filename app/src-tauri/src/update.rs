use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use murating_core::update;

#[derive(Debug, Clone, Serialize)]
pub struct Available {
    pub version: String,
    pub running: String,
    pub page: String,
    pub asset: Option<String>,
    pub size: u64,
}

static FOUND: Mutex<(Option<Available>, bool)> = Mutex::new((None, false));

pub fn start(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        let running = update::running_version();
        match update::latest() {
            Ok(Some(r)) if update::is_newer(running, &r.tag) => {
                let found = Available {
                    version: r.tag.clone(),
                    running: running.to_string(),
                    page: r.page,
                    asset: r.asset.as_ref().map(|a| a.url.clone()),
                    size: r.asset.as_ref().map(|a| a.size).unwrap_or(0),
                };
                crate::diag!("[update] {running} -> {} available", r.tag);
                if let Ok(mut f) = FOUND.lock() {
                    f.0 = Some(found.clone());
                }
                let _ = app.emit("update:available", found);
            }
            Ok(Some(r)) => crate::diag!("[update] {running} is current (latest published {})", r.tag),
            Ok(None) => crate::diag!("[update] no published release to compare against"),
            Err(e) => crate::diag!("[update] check failed: {e}"),
        }
    });
}

#[tauri::command]
pub fn update_available() -> Option<Available> {
    let mut f = FOUND.lock().ok()?;
    if f.1 {
        return None;
    }
    let out = f.0.clone();
    if out.is_some() {
        f.1 = true;
    }
    out
}

#[tauri::command]
pub fn update_dismissed() {
    if let Ok(mut f) = FOUND.lock() {
        f.1 = true;
    }
}
