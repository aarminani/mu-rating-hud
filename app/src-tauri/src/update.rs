use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use murating_core::update;

#[derive(Debug, Clone, Serialize)]
pub struct Available {
    pub version: String,
    pub exe: Option<String>,
    pub exe_size: u64,
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
                    exe: r.exe.as_ref().map(|a| a.url.clone()),
                    exe_size: r.exe.as_ref().map(|a| a.size).unwrap_or(0),
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

pub const UPDATED_FLAG: &str = "--updated";

fn old_name(version: &str) -> String {
    format!("{}.old-{}", murating_core::update::EXE_NAME, version)
}

fn is_parked(name: &str) -> bool {
    name.starts_with(&format!("{}.old-", murating_core::update::EXE_NAME))
}

pub fn clean_old() {
    let Ok(cur) = std::env::current_exe() else { return };
    let Some(dir) = cur.parent() else { return };
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for e in entries.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        if is_parked(&name) {
            match std::fs::remove_file(e.path()) {
                Ok(()) => crate::diag!("[update] removed the previous exe, {name}"),
                Err(err) => crate::diag!("[update] {name} left in place: {err}"),
            }
        }
    }
}

#[tauri::command]
pub async fn update_install(app: AppHandle) -> Result<(), String> {
    let found = FOUND.lock().ok().and_then(|f| f.0.clone()).ok_or("no update is pending")?;
    let (url, size) = match (found.exe.clone(), found.exe_size) {
        (Some(u), n) if n > 0 => (u, n),
        _ => return Err("this release has no installable build".into()),
    };
    tauri::async_runtime::spawn_blocking(move || install(app, &url, size))
        .await
        .map_err(|e| e.to_string())?
}

fn install(app: AppHandle, url: &str, size: u64) -> Result<(), String> {
    let cur = std::env::current_exe().map_err(|e| e.to_string())?;
    let dir = cur.parent().ok_or("the exe has no folder")?.to_path_buf();
    let staged = dir.join(format!("{}.new", murating_core::update::EXE_NAME));

    crate::diag!("[update] downloading {size} bytes to {}", staged.display());
    if let Err(e) = murating_core::update::download_exe(url, &staged, size) {
        let _ = std::fs::remove_file(&staged);
        return Err(e.to_string());
    }

    crate::feed::stop(&app);

    let old = dir.join(old_name(murating_core::update::running_version()));
    let _ = std::fs::remove_file(&old);
    if let Err(e) = std::fs::rename(&cur, &old) {
        let _ = std::fs::remove_file(&staged);
        return Err(format!("could not move the current helper aside: {e}"));
    }
    if let Err(e) = std::fs::rename(&staged, &cur) {
        let _ = std::fs::rename(&old, &cur);
        let _ = std::fs::remove_file(&staged);
        return Err(format!("could not put the new helper in place: {e}"));
    }
    crate::diag!("[update] installed; restarting");

    if let Err(e) = std::process::Command::new(&cur).arg(UPDATED_FLAG).spawn() {
        crate::diag!("[update] installed but could not restart: {e}");
        return Err(format!("installed, but could not restart automatically: {e}"));
    }
    app.exit(0);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_parked_name_carries_the_version_it_replaced() {
        assert_eq!(old_name("1.0.1"), "trhmu.exe.old-1.0.1");
    }

    #[test]
    fn only_files_this_updater_parked_are_ever_deleted() {
        assert!(is_parked("trhmu.exe.old-1.0.1"));
        assert!(is_parked("trhmu.exe.old-0.1.0"));

        for keep in [
            "trhmu.exe",
            "trhmu.exe.new",
            "trhmu-dash.exe",
            "trhmu-preview.exe",
            "README.txt",
            "settings.json",
            "trhmu.old",
            "old-trhmu.exe",
            "notes.exe.old-mine",
        ] {
            assert!(!is_parked(keep), "{keep} is not ours to delete");
        }
    }
}
