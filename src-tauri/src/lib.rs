mod address;
mod chains;
mod commands;
mod db;
mod error;
mod prices;
mod state;
mod sync;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running Chainfolio");
}
