#![cfg_attr(feature = "demo", allow(dead_code))]

use tauri::Manager;

mod accounts;
mod achievements;
mod commands;
#[cfg(feature = "demo")]
mod demo;
#[cfg(feature = "dash")]
mod dash;
mod diag;
mod feed;
mod follow;
mod launch;
mod process;
mod roster;
mod single;
mod toast;
mod tray;
mod window;

pub fn run() {
    let context = tauri::generate_context!();
    #[cfg(feature = "demo")]
    if !context.config().identifier.ends_with(".preview") {
        eprintln!("the preview build needs tauri.preview.conf.json: build it with npm run app:build:preview");
        return;
    }
    if !single::claim() {
        return;
    }
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec![launch::AUTOSTART_FLAG]),
        ))
        .setup(|app| {
            diag::init(app.handle());
            single::serve(app.handle());
            feed::init(app.handle());
            achievements::init(app.handle());
            roster::init(app.handle());
            tray::init(app.handle())?;
            #[cfg(feature = "dash")]
            dash::start(app.handle());
            #[cfg(not(feature = "demo"))]
            tray::refresh_autostart_path(app.handle());
            #[cfg(not(feature = "demo"))]
            follow::start(app.handle());
            #[cfg(feature = "demo")]
            demo::start(app.handle());
            if !launch::is_autostart() {
                tray::show_main(app.handle());
            }
            Ok(())
        })
        .on_window_event(|window, event| match (window.label(), event) {
            ("main", tauri::WindowEvent::CloseRequested { .. }) => tray::closing_main(window.app_handle()),
            ("main", tauri::WindowEvent::Resized(_)) => window::on_resized(window.app_handle()),
            ("main", tauri::WindowEvent::Destroyed) => window::on_destroyed(),
            ("toast", tauri::WindowEvent::Destroyed) => toast::on_destroyed(),
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            commands::probe,
            commands::connect,
            commands::exit_app,
            commands::settings,
            commands::set_setting,
            commands::accounts_list,
            commands::switch_account,
            commands::follow_state,
            commands::start_tekken,
            commands::last_polled,
            commands::disconnect_all,
            commands::troubleshoot_status,
            commands::diagnostics,
            commands::send_report,
            commands::open_table_folder,
            commands::open_mod_folder,
            commands::test_notification,
            commands::central_at,
            commands::feed_loading,
            commands::reconnect_feed,
            commands::resume_follow,
            commands::follow_now,
            commands::session_summary,
            commands::achievements,
            commands::achievements_seen,
            commands::set_utc_offset,
            window::set_window_mode,
            window::main_window_ready,
            toast::toast_ready,
            roster::roster_status,
        ])
        .build(context)
        .expect("error while building the Mu Rating HUD")
        .run(|_app, event| {
            if let tauri::RunEvent::ExitRequested { api, code: None, .. } = event {
                api.prevent_exit();
            }
        });
}
