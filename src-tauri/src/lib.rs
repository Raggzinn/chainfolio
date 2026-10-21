mod address;
mod chains;
mod commands;
mod db;
mod error;
#[cfg(test)]
mod live_tests;
mod prices;
mod security;
mod state;
mod sync;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default();

    // Self-update from GitHub Releases (signed `latest.json`) and relaunch afterwards.
    #[cfg(desktop)]
    let builder = builder
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init());

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
            Ok(())
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running Chainfolio");
}
