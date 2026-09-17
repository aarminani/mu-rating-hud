use std::sync::Mutex;

use tauri::{
    image::Image,
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager,
};

use serde::Serialize;
#[cfg(not(feature = "demo"))]
use tauri_plugin_autostart::ManagerExt;

fn autostart_enabled(app: &AppHandle) -> bool {
    #[cfg(feature = "demo")]
    {
        let _ = app;
        crate::demo::autostart()
    }
    #[cfg(not(feature = "demo"))]
    {
        app.autolaunch().is_enabled().unwrap_or(false)
    }
}

#[cfg(not(feature = "demo"))]
pub fn refresh_autostart_path(app: &AppHandle) {
    let m = app.autolaunch();
    if m.is_enabled().unwrap_or(false) {
        let _ = m.enable();
    }
}

fn set_autostart(app: &AppHandle, on: bool) {
    #[cfg(feature = "demo")]
    {
        let _ = app;
        crate::demo::set_autostart(on);
    }
    #[cfg(not(feature = "demo"))]
    {
        let m = app.autolaunch();
        let _ = if on { m.enable() } else { m.disable() };
    }
}

#[derive(Serialize, Clone)]
pub struct Settings {
    pub mr_enabled: bool,
    pub auto_hide: bool,
    pub start_with_windows: bool,
    pub hide_notifications: bool,
    pub auto_connect: bool,
    pub fighter_select: bool,
    pub pager_auto_scroll: bool,
    pub over_game: bool,
    pub headless: bool,
}

pub struct Session {
    pub name: Option<String>,
    pub tekken_id: Option<String>,
    pub payload: Option<String>,
    pub mr_enabled: bool,
    pub auto_hide: bool,
    pub hide_notifications: bool,
    pub auto_connect: bool,
    pub fighter_select: bool,
    pub pager_auto_scroll: bool,
    pub over_game: bool,
    pub headless: bool,
}

impl Default for Session {
    fn default() -> Self {
        Self {
            name: None,
            tekken_id: None,
            payload: None,
            mr_enabled: true,
            auto_hide: true,
            hide_notifications: false,
            auto_connect: true,
            fighter_select: false,
            pager_auto_scroll: true,
            over_game: true,
            headless: false,
        }
    }
}

fn settings_file(app: &AppHandle) -> Option<std::path::PathBuf> {
    app.path().app_local_data_dir().ok().map(|d| d.join("settings.json"))
}

fn load_flag(app: &AppHandle, key: &str, default: bool) -> bool {
    settings_file(app)
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| v.get(key).and_then(|b| b.as_bool()))
        .unwrap_or(default)
}

static SETTINGS_LOCK: Mutex<()> = Mutex::new(());

pub fn load_value(app: &AppHandle, key: &str) -> Option<serde_json::Value> {
    settings_file(app)
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| v.get(key).cloned())
}

pub fn save_values(app: &AppHandle, pairs: &[(&str, serde_json::Value)]) {
    let Some(p) = settings_file(app) else { return };
    let _g = SETTINGS_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(dir) = p.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let mut obj = std::fs::read_to_string(&p)
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default();
    for (k, v) in pairs {
        obj.insert(k.to_string(), v.clone());
    }
    let tmp = p.with_extension("json.tmp");
    if std::fs::write(&tmp, serde_json::Value::Object(obj).to_string()).is_ok() {
        let _ = std::fs::rename(&tmp, &p);
    }
}

fn save_flags(app: &AppHandle) {
    let s = settings(app);
    save_values(
        app,
        &[
            ("auto_hide", s.auto_hide.into()),
            ("hide_notifications", s.hide_notifications.into()),
            ("auto_connect", s.auto_connect.into()),
            ("fighter_select", s.fighter_select.into()),
            ("pager_auto_scroll", s.pager_auto_scroll.into()),
            ("over_game", s.over_game.into()),
            ("headless", s.headless.into()),
        ],
    );
}

fn session_flag(app: &AppHandle, f: impl Fn(&Session) -> bool, default: bool) -> bool {
    app.try_state::<Mutex<Session>>()
        .and_then(|s| s.lock().ok().map(|s| f(&s)))
        .unwrap_or(default)
}

pub fn auto_connect(app: &AppHandle) -> bool {
    session_flag(app, |s| s.auto_connect || s.headless, true)
}

pub fn headless(app: &AppHandle) -> bool {
    session_flag(app, |s| s.headless, false)
}

pub fn notifications_hidden(app: &AppHandle) -> bool {
    app.try_state::<Mutex<Session>>()
        .and_then(|s| s.lock().ok().map(|s| s.hide_notifications))
        .unwrap_or(false)
}

pub fn over_game(app: &AppHandle) -> bool {
    session_flag(app, |s| s.over_game, true)
}

pub fn settings(app: &AppHandle) -> Settings {
    Settings {
        mr_enabled: mr_enabled(app),
        auto_hide: auto_hide(app),
        start_with_windows: autostart_enabled(app),
        hide_notifications: notifications_hidden(app),
        auto_connect: auto_connect(app),
        fighter_select: session_flag(app, |s| s.fighter_select, false),
        pager_auto_scroll: session_flag(app, |s| s.pager_auto_scroll, true),
        over_game: session_flag(app, |s| s.over_game, true),
        headless: headless(app),
    }
}

pub fn set_setting(app: &AppHandle, key: &str, on: bool) {
    match key {
        "mr" => {
            if let Some(state) = app.try_state::<Mutex<Session>>() {
                if let Ok(mut s) = state.lock() {
                    s.mr_enabled = on;
                }
            }
            apply_mr(app);
        }
        "autohide" => {
            if let Some(state) = app.try_state::<Mutex<Session>>() {
                if let Ok(mut s) = state.lock() {
                    s.auto_hide = on;
                }
            }
            save_flags(app);
        }
        "notifications" => {
            if let Some(state) = app.try_state::<Mutex<Session>>() {
                if let Ok(mut s) = state.lock() {
                    s.hide_notifications = on;
                }
            }
            save_flags(app);
        }
        "autostart" => set_autostart(app, on),
        "headless" => {
            set_headless(app, on);
            return;
        }
        "autoconnect" | "fighter" | "pagerscroll" | "overgame" => {
            if let Some(state) = app.try_state::<Mutex<Session>>() {
                if let Ok(mut s) = state.lock() {
                    match key {
                        "autoconnect" => s.auto_connect = on,
                        "fighter" => s.fighter_select = on,
                        "pagerscroll" => s.pager_auto_scroll = on,
                        _ => s.over_game = on,
                    }
                }
            }
            save_flags(app);
            if key == "autoconnect" {
                crate::follow::poke(app);
            }
        }
        _ => return,
    }
    let _ = rebuild(app);
    let _ = app.emit("settings:changed", settings(app));
}

pub fn auto_hide(app: &AppHandle) -> bool {
    app.try_state::<Mutex<Session>>()
        .and_then(|s| s.lock().ok().map(|s| s.auto_hide))
        .unwrap_or(true)
}

pub fn set_account(app: &AppHandle, name: &str, tekken_id: &str, payload: &str) {
    if let Some(state) = app.try_state::<Mutex<Session>>() {
        if let Ok(mut s) = state.lock() {
            s.name = Some(name.to_string());
            s.tekken_id = Some(tekken_id.to_string());
            s.payload = Some(payload.to_string());
        }
    }
    let _ = rebuild(app);
}

pub fn mr_enabled(app: &AppHandle) -> bool {
    app.try_state::<Mutex<Session>>()
        .and_then(|s| s.lock().ok().map(|s| s.mr_enabled))
        .unwrap_or(true)
}

pub fn set_payload(app: &AppHandle, payload: &str) {
    if let Some(state) = app.try_state::<Mutex<Session>>() {
        if let Ok(mut s) = state.lock() {
            s.payload = Some(payload.to_string());
        }
    }
}

pub fn payload(app: &AppHandle) -> Option<String> {
    app.try_state::<Mutex<Session>>().and_then(|s| s.lock().ok().and_then(|s| s.payload.clone()))
}

pub fn current_tekken_id(app: &AppHandle) -> Option<String> {
    app.try_state::<Mutex<Session>>()
        .and_then(|s| s.lock().ok().and_then(|s| s.tekken_id.clone()))
}

pub fn clear_account(app: &AppHandle) {
    if let Some(state) = app.try_state::<Mutex<Session>>() {
        if let Ok(mut s) = state.lock() {
            s.name = None;
            s.tekken_id = None;
            s.payload = None;
        }
    }
    let _ = rebuild(app);
}

pub fn show_main(app: &AppHandle) {
    crate::window::open_main(app, None);
}

pub fn closing_main(app: &AppHandle) {
    if !headless(app) {
        crate::toast::minimized(app);
    }
}

pub fn set_headless(app: &AppHandle, on: bool) {
    if let Some(state) = app.try_state::<Mutex<Session>>() {
        if let Ok(mut s) = state.lock() {
            if s.headless == on {
                return;
            }
            s.headless = on;
        }
    }
    save_flags(app);
    if on {
        set_autostart(app, true);
    }
    apply_headless(app);
    crate::follow::poke(app);
    let _ = rebuild(app);
    let _ = app.emit("settings:changed", settings(app));
}

fn apply_headless(app: &AppHandle) {
    let on = headless(app);
    if let Some(tray) = app.tray_by_id("main") {
        let icon: &[u8] = if on {
            include_bytes!("../icons/tray-headless.png")
        } else {
            include_bytes!("../icons/tray.png")
        };
        if let Ok(image) = Image::from_bytes(icon) {
            let _ = tray.set_icon(Some(image));
        }
        let _ = tray.set_tooltip(Some(if on {
            "Tekken Resource Hub Mu Rating HUD (headless) - click to open"
        } else {
            "Tekken Resource Hub Mu Rating HUD"
        }));
        let _ = tray.set_show_menu_on_left_click(!on);
    }
}

fn apply_mr(app: &AppHandle) {
    crate::feed::wake(app);
}

fn menu(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let (name, id, enabled) = app
        .try_state::<Mutex<Session>>()
        .and_then(|s| {
            s.lock().ok().map(|s| {
                (
                    s.name.clone(),
                    s.tekken_id.clone(),
                    s.mr_enabled,
                )
            })
        })
        .unwrap_or((None, None, true));

    let dash = "\u{2014}".to_string();
    let name_item = MenuItem::with_id(
        app,
        "name",
        name.unwrap_or_else(|| dash.clone()),
        false,
        None::<&str>,
    )?;
    let id_item = MenuItem::with_id(app, "id", id.unwrap_or(dash), false, None::<&str>)?;

    let mr = CheckMenuItem::with_id(app, "mr", "Enable MR", true, enabled, None::<&str>)?;

    let autostart = CheckMenuItem::with_id(
        app,
        "autostart",
        "Start with Windows",
        true,
        autostart_enabled(app),
        None::<&str>,
    )?;

    let notifications = CheckMenuItem::with_id(
        app,
        "notifications",
        "Hide Notifications",
        true,
        notifications_hidden(app),
        None::<&str>,
    )?;

    let autohide = CheckMenuItem::with_id(
        app,
        "autohide",
        "Auto-hide Helper",
        true,
        auto_hide(app),
        None::<&str>,
    )?;

    let autoconnect = CheckMenuItem::with_id(
        app,
        "autoconnect",
        "Auto-connect",
        true,
        auto_connect(app),
        None::<&str>,
    )?;

    let ratings = MenuItem::with_id(app, "ratings", "Open Helper", true, None::<&str>)?;

    let change = MenuItem::with_id(app, "change", "Change Account", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Exit", true, None::<&str>)?;

    Menu::with_items(
        app,
        &[
            &name_item,
            &id_item,
            &PredefinedMenuItem::separator(app)?,
            &mr,
            &autoconnect,
            &autohide,
            &autostart,
            &notifications,
            &PredefinedMenuItem::separator(app)?,
            &ratings,
            &change,
            &quit,
        ],
    )
}

fn rebuild(app: &AppHandle) -> tauri::Result<()> {
    if let Some(tray) = app.tray_by_id("main") {
        tray.set_menu(Some(menu(app)?))?;
    }
    Ok(())
}

fn on_menu(app: &AppHandle, id: &str) {
    match id {
        "ratings" => crate::window::open_main(app, Some("app:show-ratings")),
        "change" => crate::window::open_main(app, Some("app:change-account")),
        "mr" => set_setting(app, "mr", !mr_enabled(app)),
        "autohide" => set_setting(app, "autohide", !auto_hide(app)),
        "autostart" => {
            let on = autostart_enabled(app);
            set_setting(app, "autostart", !on);
        }
        "notifications" => set_setting(app, "notifications", !notifications_hidden(app)),
        "autoconnect" => set_setting(app, "autoconnect", !auto_connect(app)),
        "quit" => app.exit(0),
        _ => {}
    }
}

pub fn init(app: &AppHandle) -> tauri::Result<()> {
    app.manage(Mutex::new(Session {
        auto_hide: load_flag(app, "auto_hide", true),
        hide_notifications: load_flag(app, "hide_notifications", false),
        auto_connect: load_flag(app, "auto_connect", true),
        fighter_select: load_flag(app, "fighter_select", false),
        pager_auto_scroll: load_flag(app, "pager_auto_scroll", true),
        over_game: load_flag(app, "over_game", true),
        headless: load_flag(app, "headless", false),
        ..Session::default()
    }));

    TrayIconBuilder::with_id("main")
        .icon(Image::from_bytes(include_bytes!("../icons/tray.png"))?)
        .tooltip("Tekken Resource Hub Mu Rating HUD")
        .menu(&menu(app)?)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| on_menu(app, event.id.as_ref()))
        .on_tray_icon_event(|tray, event| {
            if let tauri::tray::TrayIconEvent::Click {
                button: tauri::tray::MouseButton::Left,
                button_state: tauri::tray::MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if headless(app) {
                    crate::window::open_main(app, Some("app:show-ratings"));
                }
            }
        })
        .build(app)?;
    apply_headless(app);

    Ok(())
}
