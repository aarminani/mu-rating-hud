use serde::Serialize;

use murating_core::{characters, game, paths, player, slot, steam, table};

use crate::{accounts, feed, toast, tray};

pub(crate) const USER_AGENT: &str =
    concat!("MuRatingHelper/", env!("CARGO_PKG_VERSION"), " (+https://tekkenresourcehub.com; Tekken 8 MR badge)");

const BASE_URL: &str = "https://wank.wavu.wiki";

#[derive(Serialize)]
pub struct Probe {
    pub saves_root: String,
    pub saves_root_exists: bool,
    pub account: Option<String>,
    pub slot_exists: bool,
    pub slot_is_ours: bool,
    pub build_id: Option<u64>,
    pub autostart: bool,
}

#[tauri::command]
pub fn exit_app(app: tauri::AppHandle) {
    app.exit(0);
}

#[tauri::command]
pub fn settings(app: tauri::AppHandle) -> tray::Settings {
    tray::settings(&app)
}

#[tauri::command]
pub fn set_setting(app: tauri::AppHandle, key: String, on: bool) -> tray::Settings {
    tray::set_setting(&app, &key, on);
    tray::settings(&app)
}

#[tauri::command]
pub async fn probe() -> Probe {
    tauri::async_runtime::spawn_blocking(probe_now)
        .await
        .expect("probe task")
}

fn probe_now() -> Probe {
    let root = paths::saves_root();
    let exists = root.is_dir();
    let account = paths::active_account(&root).ok();
    let p = paths::slot_dir().join(slot::SLOT_FILE);

    let (slot_exists, slot_is_ours) = if p.is_file() {
        let head: Vec<u8> = std::fs::read(&p)
            .map(|b| b.into_iter().take(4).collect())
            .unwrap_or_default();
        (true, head == b"GVAS")
    } else {
        (false, false)
    };

    Probe {
        saves_root: root.to_string_lossy().to_string(),
        saves_root_exists: exists,
        account: account.map(|d| paths::account_id(&d)),
        slot_exists,
        slot_is_ours,
        build_id: game::build_id(),
        autostart: crate::launch::is_autostart(),
    }
}

#[derive(Serialize, Clone, PartialEq, Debug)]
pub struct CharRating {
    pub character: String,
    pub mu: i32,
    pub games: Option<i32>,
    pub last_seen: Option<i64>,
    pub group: String,
    pub change: Option<i32>,
    pub sigma: Option<i32>,
}

pub(crate) fn char_ratings(live: &[murating_core::feed::LiveRating]) -> Vec<CharRating> {
    live.iter()
        .map(|l| CharRating {
            character: characters::full_name(&l.rating.character),
            mu: l.rating.mu,
            games: l.rating.games,
            last_seen: l.rating.last_seen,
            group: l.rating.group.clone(),
            change: l.change,
            sigma: l.rating.sigma,
        })
        .collect()
}

#[derive(Serialize, Clone, PartialEq, Debug)]
pub struct Connected {
    pub tekken_id: String,
    pub name: String,
    pub character: String,
    pub mu: i32,
    pub recent_character: String,
    pub recent_mu: i32,
    pub eligible: bool,
    pub ratings: Vec<CharRating>,
    pub wrote: String,
}

pub enum Announce {
    Established,
    Switched,
    Quiet,
}

static ACTIVATE: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[derive(Serialize, Clone)]
struct Progress<'a> {
    tekken_id: &'a str,
    state: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

pub(crate) fn progress(app: &tauri::AppHandle, tekken_id: &str, state: &str, name: Option<String>, error: Option<String>) {
    use tauri::Emitter;
    let _ = app.emit("connect:progress", Progress { tekken_id, state, name, error });
}

const PAGE_SPACING: std::time::Duration = std::time::Duration::from_secs(10);

fn fetch_page(id: &str) -> Result<String, String> {
    let (page, ms) = crate::diag::timed(|| player::fetch(BASE_URL, id, USER_AGENT));
    match &page {
        Ok(_) => crate::diag::net_ok(crate::diag::Endpoint::Wavu, ms, None),
        Err(player::PlayerError::NotFound) => crate::diag::net_ok(crate::diag::Endpoint::Wavu, ms, None),
        Err(e) => crate::diag::net_err(crate::diag::Endpoint::Wavu, format!("player page: {e}")),
    }
    page.map_err(|e| match e {
        player::PlayerError::NotFound => format!("No Wavu Wank player with the Tekken ID {id}."),
        other => other.to_string(),
    })
}

pub(crate) fn activate(
    app: &tauri::AppHandle,
    id: &str,
    html: Option<String>,
    announce: Announce,
) -> Result<Connected, String> {
    #[cfg(feature = "demo")]
    if true {
        let _ = html;
        return crate::demo::activate(app, id, announce);
    }
    let _one = ACTIVATE.lock().unwrap_or_else(|e| e.into_inner());
    let id = id.to_string();
    let html = match html {
        Some(h) => h,
        None => fetch_page(&id)?,
    };

    feed::polled(app, feed::now());
    crate::roster::observe_page(app, &html);
    let ratings = player::parse_ratings(&html).map_err(|e| e.to_string())?;
    let cur = player::current(&ratings)
        .ok_or_else(|| "No rated character on that account yet.".to_string())?;
    let top = player::best(&ratings).unwrap_or(cur);
    let name = player::parse_name(&html).unwrap_or_else(|| id.clone());

    let payload = format!("{} MR", cur.mu);
    let own = murating_core::feed::name_value(&mut [(0, payload.clone())]).unwrap_or_default();
    let blob = slot::replay_bytes(&payload, &[(name.clone(), own)], &[]);
    let on_disk = std::fs::metadata(paths::slot_dir().join(slot::SLOT_FILE)).map(|m| m.len()).unwrap_or(0);
    let wrote = if crate::process::game_running() || (blob.len() as u64) < on_disk {
        String::new()
    } else {
        slot::write_slot(&paths::slot_dir(), &blob)
            .map_err(|e| e.to_string())?
            .to_string_lossy()
            .to_string()
    };

    match announce {
        Announce::Established => toast::established(app, &name),
        Announce::Switched => toast::switched(app, &name),
        Announce::Quiet => {}
    }
    tray::set_account(app, &name, &id, &payload);
    feed::start(
        app,
        id.clone(),
        name.clone(),
        payload.clone(),
        player::parse_battle_times(&html),
        ratings.clone(),
        player::parse_history(&html),
    );

    accounts::refresh(
        app,
        &id,
        &name,
        player::parse_steam_id(&html),
        Some((top.character.clone(), top.mu)),
    );

    let connected = Connected {
        tekken_id: id,
        name,
        character: characters::full_name(&top.character),
        mu: top.mu,
        recent_character: characters::full_name(&cur.character),
        recent_mu: cur.mu,
        eligible: true,
        ratings: ratings
            .iter()
            .map(|r| CharRating {
                character: characters::full_name(&r.character),
                mu: r.mu,
                games: r.games,
                last_seen: r.last_seen,
                group: r.group.clone(),
                change: None,
                sigma: r.sigma,
            })
            .collect(),
        wrote,
    };
    crate::follow::set_following(app, connected.clone());
    Ok(connected)
}

#[tauri::command]
pub async fn connect(app: tauri::AppHandle, tekken_ids: Vec<String>, prefer: Option<String>) -> Result<Connected, String> {
    tauri::async_runtime::spawn_blocking(move || connect_all(&app, &tekken_ids, prefer))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn accounts_list(app: tauri::AppHandle) -> Vec<accounts::AccountRow> {
    tauri::async_runtime::spawn_blocking(move || accounts::rows(&app))
        .await
        .unwrap_or_default()
}

#[tauri::command]
pub async fn switch_account(app: tauri::AppHandle, tekken_id: String) -> Result<Connected, String> {
    tauri::async_runtime::spawn_blocking(move || {
        crate::follow::pin(&tekken_id);
        activate(&app, &tekken_id, None, Announce::Quiet)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn start_tekken(app: tauri::AppHandle) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_url(format!("steam://rungameid/{}", game::APP_ID), None::<&str>)
        .map_err(|e| e.to_string())?;
    crate::follow::poke(&app);
    Ok(())
}

#[tauri::command]
pub async fn troubleshoot_status(app: tauri::AppHandle) -> crate::diag::Status {
    tauri::async_runtime::spawn_blocking(move || crate::diag::status(&app))
        .await
        .expect("status task")
}

#[tauri::command]
pub async fn diagnostics(app: tauri::AppHandle) -> String {
    tauri::async_runtime::spawn_blocking(move || crate::diag::text(&app))
        .await
        .unwrap_or_default()
}

#[tauri::command]
pub async fn send_report(app: tauri::AppHandle, note: Option<String>) -> Result<crate::diag::Sent, String> {
    tauri::async_runtime::spawn_blocking(move || crate::diag::send(&app, note))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn open_table_folder(app: tauri::AppHandle) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    crate::follow::pause(&app);
    let dir = paths::slot_dir();
    let file = dir.join(slot::SLOT_FILE);
    let opened = if file.is_file() {
        app.opener().reveal_item_in_dir(&file)
    } else {
        app.opener().open_path(dir.to_string_lossy(), None::<&str>)
    };
    opened.map_err(|e| e.to_string())
}

#[tauri::command]
pub fn test_notification(app: tauri::AppHandle) -> u64 {
    crate::toast::test(&app);
    crate::toast::TEST_DELAY_SECS
}

#[tauri::command]
pub fn open_mod_folder(app: tauri::AppHandle) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    let paks = game::paks_dir().ok_or_else(|| "Tekken 8 install not found".to_string())?;
    let files = game::mod_files();
    let found = files
        .iter()
        .find(|f| f.path.ends_with(".pak"))
        .or(files.first())
        .map(|f| paks.join(f.path.replace('/', "\\")))
        .filter(|p| p.is_file());
    let opened = match found {
        Some(file) => app.opener().reveal_item_in_dir(&file),
        None => app.opener().open_path(paks.to_string_lossy(), None::<&str>),
    };
    opened.map_err(|e| e.to_string())
}

#[tauri::command]
pub fn resume_follow(app: tauri::AppHandle) {
    crate::follow::resume(&app);
}

#[tauri::command]
pub fn last_polled() -> Option<i64> {
    feed::last_polled()
}

#[tauri::command]
pub fn central_at() -> Option<i64> {
    feed::last_central_at()
}

#[tauri::command]
pub fn feed_loading() -> bool {
    feed::is_central_loading()
}

#[tauri::command]
pub fn reconnect_feed(app: tauri::AppHandle) -> bool {
    feed::refetch(&app)
}

#[tauri::command]
pub async fn disconnect_all(app: tauri::AppHandle) {
    let _ = tauri::async_runtime::spawn_blocking(move || {
        let _one = ACTIVATE.lock().unwrap_or_else(|e| e.into_inner());
        crate::follow::disconnect(&app);
    })
    .await;
}

#[tauri::command]
pub fn follow_state() -> Option<crate::follow::FollowState> {
    crate::follow::known_state()
}

#[tauri::command]
pub fn follow_now() {
    crate::follow::connect_now();
}

#[tauri::command]
pub fn session_summary(app: tauri::AppHandle) -> Option<murating_core::session::SessionSummary> {
    feed::session(&app)
}

#[tauri::command]
pub fn achievements(app: tauri::AppHandle) -> Option<crate::achievements::Snapshot> {
    crate::achievements::snapshot(&app)
}

#[tauri::command]
pub fn achievements_seen(app: tauri::AppHandle) -> Option<crate::achievements::Snapshot> {
    crate::achievements::mark_seen(&app)
}

#[tauri::command]
pub fn set_utc_offset(app: tauri::AppHandle, minutes: i32) {
    crate::achievements::set_utc_offset(&app, minutes);
}

fn connect_all(app: &tauri::AppHandle, entered: &[String], prefer: Option<String>) -> Result<Connected, String> {
    #[cfg(feature = "demo")]
    if true {
        return crate::demo::connect_all(app, entered, prefer);
    }
    let mut ids: Vec<String> = Vec::new();
    for raw in entered {
        let id = table::tekken_id(raw);
        if id.is_empty() || ids.contains(&id) {
            continue;
        }
        if !table::looks_like_tekken_id(&id) {
            return Err(format!(
                "{id:?} is not a Tekken ID, it should be 12 letters and digits, as shown in the game's Replay menu"
            ));
        }
        ids.push(id);
    }
    let Some(first) = ids.first().cloned() else {
        return Err("Enter a Tekken ID.".to_string());
    };

    let saved = accounts::load(app);
    let mut pages = std::collections::HashMap::new();
    let mut last_fetch: Option<std::time::Instant> = None;
    let mut paced = |id: &str| -> Result<String, String> {
        if let Some(t) = last_fetch {
            let waited = t.elapsed();
            if waited < PAGE_SPACING {
                std::thread::sleep(PAGE_SPACING - waited);
            }
        }
        let page = fetch_page(id);
        last_fetch = Some(std::time::Instant::now());
        page
    };

    for id in &ids {
        if let Some(a) = saved.iter().find(|a| &a.tekken_id == id && a.steam_id.is_some()) {
            progress(app, id, "done", Some(a.name.clone()), None);
        }
    }
    let mut list = Vec::new();
    for id in &ids {
        match saved.iter().find(|a| &a.tekken_id == id && a.steam_id.is_some()) {
            Some(a) => list.push(a.clone()),
            None => {
                progress(app, id, "reading", None, None);
                let html = match paced(id) {
                    Ok(h) => h,
                    Err(e) => {
                        progress(app, id, "failed", None, Some(e.clone()));
                        return Err(e);
                    }
                };
                let name = player::parse_name(&html).unwrap_or_else(|| id.clone());
                let best = player::parse_ratings(&html)
                    .ok()
                    .and_then(|r| player::best(&r).map(|b| (b.character.clone(), b.mu)));
                let old = saved.iter().find(|a| &a.tekken_id == id);
                list.push(accounts::Account {
                    tekken_id: id.clone(),
                    name: name.clone(),
                    steam_id: player::parse_steam_id(&html),
                    character: best.as_ref().map(|b| b.0.clone()).or_else(|| old.and_then(|a| a.character.clone())),
                    mu: best.as_ref().map(|b| b.1).or_else(|| old.and_then(|a| a.mu)),
                    followed_at: old.and_then(|a| a.followed_at),
                });
                progress(app, id, "done", Some(name), None);
                pages.insert(id.clone(), html);
            }
        }
    }
    accounts::save(app, &list);

    let signed_in = steam::active_steam_id();
    let active = match prefer.filter(|p| ids.contains(p)) {
        Some(p) => {
            crate::follow::pin(&p);
            p
        }
        None => list
            .iter()
            .find(|a| a.steam_id.is_some() && a.steam_id == signed_in)
            .map(|a| a.tekken_id.clone())
            .or_else(|| accounts::last_followed(&list))
            .unwrap_or(first),
    };
    let html = match pages.remove(&active) {
        Some(h) => h,
        None => paced(&active)?,
    };
    activate(app, &active, Some(html), Announce::Established)
}
