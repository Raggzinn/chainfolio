mod address;
mod chains;
mod commands;
mod db;
mod defi;
mod error;
#[cfg(test)]
mod live_tests;
mod prices;
mod risk;
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
            let conn = db::open(&dir.join("chainfolio.db"))?;
            app.manage(state::AppState::new(conn)?);
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
            commands::get_positions,
            commands::get_risk_report,
            commands::scan_risk,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Chainfolio");
}
