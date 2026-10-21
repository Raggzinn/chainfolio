//! Master password commands. The password itself is never stored: SQLCipher
//! derives the database key from it, so a wrong password simply can't read
//! the file.

use tauri::State;

use crate::{
    error::Result,
    state::{AppState, SecurityStatus},
};

#[tauri::command]
pub fn security_status(state: State<'_, AppState>) -> SecurityStatus {
    state.security_status()
}

/// Opens the encrypted database. Runs off the main thread because the key
/// derivation is deliberately slow.
#[tauri::command]
pub async fn unlock(state: State<'_, AppState>, password: String) -> Result<()> {
    state.unlock(&password)
}

#[tauri::command]
pub fn lock(state: State<'_, AppState>) {
    state.lock();
}

/// Sets, changes or (with `new_password` = null) removes the master password.
#[tauri::command]
pub async fn set_master_password(
    state: State<'_, AppState>,
    current_password: Option<String>,
    new_password: Option<String>,
) -> Result<()> {
    state.change_key(current_password.as_deref(), new_password.as_deref())
}
