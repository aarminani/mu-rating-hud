use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

use murating_core::steam;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Account {
    pub tekken_id: String,
    pub name: String,
    pub steam_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub character: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mu: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub followed_at: Option<i64>,
}

#[derive(Serialize, Clone, Debug)]
pub struct AccountRow {
    pub tekken_id: String,
    pub name: String,
    pub paired: bool,
    pub signed_in: bool,
    pub active: bool,
    pub character: Option<String>,
    pub mu: Option<i32>,
}

static LOCK: Mutex<()> = Mutex::new(());

fn file(app: &AppHandle) -> Option<std::path::PathBuf> {
    app.path().app_local_data_dir().ok().map(|d| d.join("accounts.json"))
}

fn read(app: &AppHandle) -> Vec<Account> {
    file(app)
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn write(app: &AppHandle, accounts: &[Account]) {
    let Some(p) = file(app) else { return };
    if let Some(dir) = p.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(json) = serde_json::to_string_pretty(accounts) {
        let tmp = p.with_extension("json.tmp");
        if std::fs::write(&tmp, json).is_ok() {
            let _ = std::fs::rename(&tmp, &p);
        }
    }
}

pub fn load(app: &AppHandle) -> Vec<Account> {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    read(app)
}

pub fn save(app: &AppHandle, accounts: &[Account]) {
    {
        let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        write(app, accounts);
    }
    changed(app);
}

fn update(app: &AppHandle, f: impl FnOnce(&mut Vec<Account>)) {
    let wrote = {
        let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut list = read(app);
        let before = list.clone();
        f(&mut list);
        let wrote = list != before;
        if wrote {
            write(app, &list);
        }
        wrote
    };
    if wrote {
        changed(app);
    }
}

pub fn set_best(app: &AppHandle, tekken_id: &str, character: &str, mu: i32) {
    update(app, |list| {
        if let Some(a) = list.iter_mut().find(|a| a.tekken_id == tekken_id) {
            a.character = Some(character.to_string());
            a.mu = Some(mu);
        }
    });
}

pub fn refresh(app: &AppHandle, tekken_id: &str, name: &str, steam_id: Option<String>, best: Option<(String, i32)>) {
    let now = crate::feed::now();
    update(app, |list| {
        if let Some(a) = list.iter_mut().find(|a| a.tekken_id == tekken_id) {
            a.name = name.to_string();
            if steam_id.is_some() {
                a.steam_id = steam_id;
            }
            if let Some((c, mu)) = best {
                a.character = Some(c);
                a.mu = Some(mu);
            }
            a.followed_at = Some(now);
        }
    });
}

pub fn last_followed(accounts: &[Account]) -> Option<String> {
    accounts
        .iter()
        .filter(|a| a.followed_at.is_some())
        .max_by_key(|a| a.followed_at)
        .map(|a| a.tekken_id.clone())
}

pub fn rows(app: &AppHandle) -> Vec<AccountRow> {
    #[cfg(feature = "demo")]
    if true {
        return crate::demo::rows(app);
    }
    let signed_in = steam::active_steam_id();
    let following = crate::tray::current_tekken_id(app);
    load(app)
        .into_iter()
        .map(|a| AccountRow {
            paired: a.steam_id.is_some(),
            signed_in: a.steam_id.is_some() && a.steam_id == signed_in && steam_up(),
            active: following.as_deref() == Some(a.tekken_id.as_str()),
            tekken_id: a.tekken_id,
            name: a.name,
            character: a.character,
            mu: a.mu,
        })
        .collect()
}

pub(crate) fn steam_up() -> bool {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    std::process::Command::new("tasklist")
        .args(["/FI", "IMAGENAME eq steam.exe", "/NH"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_ascii_lowercase().contains("steam.exe"))
        .unwrap_or(false)
}

pub fn changed(app: &AppHandle) {
    let _ = app.emit("accounts:changed", rows(app));
}
