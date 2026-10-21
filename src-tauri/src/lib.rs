mod address;
mod analytics;
mod chains;
mod commands;
mod db;
mod engine;
mod error;
mod history;
#[cfg(test)]
mod live_tests;
mod prices;
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
        .plugin(tauri_plugin_opener::init())
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
            commands::sync_history,
            commands::list_transactions,
            commands::update_transaction,
            commands::export_transactions_csv,
            commands::portfolio_history,
            commands::pnl_report,
            commands::get_api_keys,
            commands::set_api_key,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Chainfolio");
}
