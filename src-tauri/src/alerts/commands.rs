//! Tauri commands for the alerts page. Types are mirrored in `src/lib/api.ts`.

use rust_decimal::Decimal;
use serde::Serialize;
use tauri::State;

use super::{Condition, Rule, store};
use crate::{error::Result, state::AppState};

#[tauri::command]
pub fn list_alert_rules(state: State<'_, AppState>) -> Result<Vec<Rule>> {
    store::list_rules(&*state.db()?)
}

#[tauri::command]
pub fn create_alert_rule(state: State<'_, AppState>, condition: Condition) -> Result<i64> {
    store::insert_rule(&*state.db()?, &condition, true)
}

#[tauri::command]
pub fn update_alert_rule(state: State<'_, AppState>, id: i64, condition: Condition) -> Result<()> {
    store::update_rule(&*state.db()?, id, &condition)
}

#[tauri::command]
pub fn set_alert_rule_enabled(state: State<'_, AppState>, id: i64, enabled: bool) -> Result<()> {
    store::set_enabled(&*state.db()?, id, enabled)
}

#[tauri::command]
pub fn delete_alert_rule(state: State<'_, AppState>, id: i64) -> Result<()> {
    store::delete_rule(&*state.db()?, id)
}

#[tauri::command]
pub fn list_alert_events(
    state: State<'_, AppState>,
    limit: Option<u32>,
) -> Result<Vec<store::AlertEvent>> {
    store::recent_events(&*state.db()?, limit.unwrap_or(50).min(500))
}

#[tauri::command]
pub fn clear_alert_events(state: State<'_, AppState>) -> Result<()> {
    store::clear_events(&*state.db()?)
}

/// An asset a price alert can watch.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchableAsset {
    pub price_key: String,
    pub symbol: String,
    pub name: String,
    pub price_usd: Option<Decimal>,
    pub held: bool,
}

/// Majors offered even when no wallet holds them.
const MAJORS: &[(&str, &str, &str)] = &[
    ("coingecko:bitcoin", "BTC", "Bitcoin"),
    ("coingecko:ethereum", "ETH", "Ether"),
    ("coingecko:solana", "SOL", "Solana"),
    ("coingecko:binancecoin", "BNB", "BNB"),
    ("coingecko:ripple", "XRP", "XRP"),
    ("coingecko:tron", "TRX", "TRON"),
    ("coingecko:avalanche-2", "AVAX", "Avalanche"),
    ("coingecko:polygon-ecosystem-token", "POL", "Polygon"),
    ("coingecko:chainlink", "LINK", "Chainlink"),
    ("coingecko:dogecoin", "DOGE", "Dogecoin"),
];

/// Held assets first (by value), then the majors nobody holds.
#[tauri::command]
pub fn alert_assets(state: State<'_, AppState>) -> Result<Vec<WatchableAsset>> {
    let db = state.db()?;
    let mut stmt = db.prepare(
        "SELECT a.price_key, a.symbol, a.name, p.price_usd,
                SUM(CAST(b.amount AS REAL) * CAST(p.price_usd AS REAL)) AS value
         FROM balances b
         JOIN assets a ON a.id = b.asset_id
         JOIN prices p ON p.price_key = a.price_key
         GROUP BY a.price_key
         ORDER BY value DESC",
    )?;
    let mut assets: Vec<WatchableAsset> = stmt
        .query_map([], |r| {
            Ok(WatchableAsset {
                price_key: r.get(0)?,
                symbol: r.get(1)?,
                name: r.get(2)?,
                price_usd: r.get::<_, String>(3)?.parse().ok(),
                held: true,
            })
        })?
        .collect::<rusqlite::Result<_>>()?;
    let (prices, _) = store::prices(&db)?;
    for (key, symbol, name) in MAJORS {
        if assets.iter().all(|a| a.price_key != *key) {
            assets.push(WatchableAsset {
                price_key: (*key).into(),
                symbol: (*symbol).into(),
                name: (*name).into(),
                price_usd: prices.get(*key).copied(),
                held: false,
            });
        }
    }
    Ok(assets)
}
