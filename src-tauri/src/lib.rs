mod address;
mod background;
mod chains;
mod commands;
mod db;
mod error;
#[cfg(test)]
mod live_tests;
mod prices;
mod security;
mod settings;
mod state;
mod sync;
mod text;
#[cfg(desktop)]
mod tray;

use tauri::Manager;

/// Passed by the login item so the app starts hidden in the tray.
const HIDDEN_ARG: &str = "--hidden";

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default();

    // A second launch focuses the running window instead (must be registered first).
    // Self-update from GitHub Releases (signed `latest.json`) and relaunch afterwards.
    #[cfg(desktop)]
    let builder = builder
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            tray::show_main(app);
        }))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec![HIDDEN_ARG]),
        ));

    builder
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let path = dir.join("chainfolio.db");
            // An encrypted database stays closed until the lock screen unlocks it.
            let conn = if db::is_encrypted(&path)? {
                None
            } else {
                Some(db::open(&path, None)?)
            };
            app.manage(state::AppState::new(conn, path)?);
            app.manage(settings::SettingsStore::load(dir.join("settings.json")));

            #[cfg(desktop)]
            tray::build(app.handle())?;
            background::spawn(app.handle().clone());
            // The window starts hidden so a login launch can stay in the tray.
            if !std::env::args().any(|arg| arg == HIDDEN_ARG) {
                if let Some(window) = app.get_webview_window("main") {
                    window.show()?;
                }
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let settings = window.app_handle().state::<settings::SettingsStore>();
                if cfg!(desktop) && settings.get().close_to_tray {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::detect_address,
            commands::add_wallet,
            commands::list_wallets,
            commands::rename_wallet,
            commands::remove_wallet,
            commands::get_portfolio,
            commands::refresh_balances,
            security::security_status,
            security::unlock,
            security::lock,
            security::set_master_password,
            settings::get_app_settings,
            settings::update_app_settings,
            #[cfg(desktop)]
            tray::set_tray_privacy,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Chainfolio");
}
