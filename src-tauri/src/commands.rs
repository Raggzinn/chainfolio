//! Tauri commands exposed to the frontend. Argument and result types are
//! mirrored in `src/lib/api.ts`.

use tauri::State;

use crate::{
    address::{self, Detection, InputKind},
    chains::{evm, solana},
    db::history::{self as store, TxPage, TxQuery},
    db::repo::{self, NewWallet, Portfolio, WalletSummary},
    error::{Error, Result},
    history::{
        self,
        sync::{HistoryPoint, HistoryReport},
    },
    state::AppState,
    sync::{self, RefreshReport},
};

/// Classifies an address, name or extended key without touching the network.
#[tauri::command]
pub fn detect_address(input: String) -> Result<Detection> {
    address::detect(&input)
}

/// Registers a watch-only wallet. Names are resolved now so the stored address
/// never changes behind the user's back. Balances load with `refresh_balances`.
#[tauri::command]
pub async fn add_wallet(
    state: State<'_, AppState>,
    input: String,
    label: Option<String>,
) -> Result<WalletSummary> {
    let detection = address::detect(&input)?;
    let resolved = match detection.kind {
        InputKind::Ens => evm::resolve_ens(&state.http, &detection.normalized).await?,
        InputKind::Sns => solana::resolve_sns(&state.http, &detection.normalized).await?,
        InputKind::Address | InputKind::Xpub => detection.normalized.clone(),
    };
    let label = label
        .filter(|l| !l.trim().is_empty())
        .unwrap_or_else(|| default_label(&detection, &resolved));

    let mut db = state.db();
    let wallet = repo::insert_wallet(
        &mut db,
        &NewWallet {
            label: &label,
            family: detection.family,
            kind: detection.kind.into(),
            address: &resolved,
            input: &detection.normalized,
        },
    )?;
    repo::portfolio(&db)?
        .wallets
        .into_iter()
        .find(|w| w.id == wallet.id)
        .ok_or(Error::NotFound)
}

fn default_label(detection: &Detection, resolved: &str) -> String {
    match detection.kind {
        InputKind::Ens | InputKind::Sns => detection.normalized.clone(),
        InputKind::Address | InputKind::Xpub => {
            let head: String = resolved.chars().take(6).collect();
            let tail: String = resolved
                .chars()
                .skip(resolved.chars().count().saturating_sub(4))
                .collect();
            format!("{head}…{tail}")
        }
    }
}

#[tauri::command]
pub fn list_wallets(state: State<'_, AppState>) -> Result<Vec<WalletSummary>> {
    Ok(repo::portfolio(&state.db())?.wallets)
}

#[tauri::command]
pub fn rename_wallet(state: State<'_, AppState>, id: i64, label: String) -> Result<()> {
    repo::rename_wallet(&state.db(), id, &label)
}

#[tauri::command]
pub fn remove_wallet(state: State<'_, AppState>, id: i64) -> Result<()> {
    repo::remove_wallet(&state.db(), id)
}

/// Portfolio from the local cache; instant and offline.
#[tauri::command]
pub fn get_portfolio(state: State<'_, AppState>) -> Result<Portfolio> {
    repo::portfolio(&state.db())
}

/// Fetches fresh balances and prices for one wallet, or all when `wallet_id`
/// is omitted.
#[tauri::command]
pub async fn refresh_balances(
    state: State<'_, AppState>,
    wallet_id: Option<i64>,
) -> Result<RefreshReport> {
    sync::refresh(&state, wallet_id).await
}

/// Pulls new transactions (and a bit more of the older backlog) for one
/// wallet, or all when `wallet_id` is omitted.
#[tauri::command]
pub async fn sync_history(
    state: State<'_, AppState>,
    wallet_id: Option<i64>,
) -> Result<HistoryReport> {
    history::sync::sync(&state, wallet_id).await
}

#[tauri::command]
pub fn list_transactions(state: State<'_, AppState>, query: TxQuery) -> Result<TxPage> {
    store::list_transactions(&state.db(), &query)
}

/// Overrides the category of a wallet's view of a transaction and/or sets a
/// note. `None` clears either.
#[tauri::command]
pub fn update_transaction(
    state: State<'_, AppState>,
    tx_id: String,
    wallet_id: i64,
    kind: Option<String>,
    note: Option<String>,
) -> Result<()> {
    store::update_entry(
        &state.db(),
        &tx_id,
        wallet_id,
        kind.as_deref(),
        note.as_deref(),
    )
}

/// CSV of every transaction matching `query`.
#[tauri::command]
pub fn export_transactions_csv(state: State<'_, AppState>, query: TxQuery) -> Result<String> {
    store::transactions_csv(&state.db(), &query)
}

/// Portfolio value over time, optionally for some assets only.
#[tauri::command]
pub async fn portfolio_history(
    state: State<'_, AppState>,
    from: i64,
    to: i64,
    asset_ids: Option<Vec<String>>,
) -> Result<Vec<HistoryPoint>> {
    history::sync::portfolio_history(&state, from, to, asset_ids).await
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiKeyStatus {
    pub etherscan: bool,
}

/// Which optional provider keys are set. Values never leave the backend.
#[tauri::command]
pub fn get_api_keys(state: State<'_, AppState>) -> Result<ApiKeyStatus> {
    Ok(ApiKeyStatus {
        etherscan: store::get_setting(&state.db(), history::sync::ETHERSCAN_KEY)?.is_some(),
    })
}

/// Stores (or clears, with an empty value) an optional provider key.
#[tauri::command]
pub fn set_api_key(
    state: State<'_, AppState>,
    provider: String,
    key: Option<String>,
) -> Result<()> {
    let setting = match provider.as_str() {
        "etherscan" => history::sync::ETHERSCAN_KEY,
        _ => return Err(Error::provider(format!("unknown provider {provider}"))),
    };
    store::set_setting(&state.db(), setting, key.as_deref())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_labels() {
        let ens = address::detect("vitalik.eth").unwrap();
        assert_eq!(default_label(&ens, "0xd8dA…"), "vitalik.eth");
        let evm = address::detect("0xd8da6bf26964af9d7eed9e03e53415d37aa96045").unwrap();
        assert_eq!(default_label(&evm, &evm.normalized), "0xd8dA…6045");
    }
}
