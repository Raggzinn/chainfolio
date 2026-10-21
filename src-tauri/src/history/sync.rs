//! Keeps local transaction history up to date and builds portfolio charts.

use std::collections::{HashMap, HashSet};

use rust_decimal::Decimal;
use serde::Serialize;

use super::{ApiKeys, Page, RawTx, Unavailable, adapters};
use crate::{
    chains::Chain,
    db::{
        history::{self as store, Cursor},
        repo,
    },
    engine::replay,
    error::Result,
    prices::defillama::DefiLlama,
    state::AppState,
};

/// Pages read per chain on an incremental sync before giving up on reaching
/// already-stored history (very active wallets).
const NEW_PAGES: usize = 10;
/// Pages read on a wallet's very first sync, so it shows up quickly.
const FIRST_PAGES: usize = 2;
/// Older pages fetched per chain and sync while backfilling.
const BACKFILL_PAGES: usize = 3;
/// Oldest history the charts and transaction values look back.
const MAX_DAYS: i64 = 730;
const DAY_MS: i64 = 86_400_000;

pub const ETHERSCAN_KEY: &str = "etherscan_api_key";

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryIssue {
    pub wallet_id: i64,
    pub chain: Chain,
    pub message: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MissingSource {
    pub wallet_id: i64,
    pub chain: Chain,
    pub reason: &'static str,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryReport {
    pub new_transactions: usize,
    pub issues: Vec<HistoryIssue>,
    pub missing: Vec<MissingSource>,
    /// True when some chain still has older history left to backfill.
    pub backfilling: bool,
}

fn keys(state: &AppState) -> Result<ApiKeys> {
    Ok(ApiKeys {
        etherscan: store::get_setting(&state.db(), ETHERSCAN_KEY)?,
    })
}

/// Fetches new transactions for one wallet (or all) and continues
/// backfilling older history a few pages at a time.
pub async fn sync(state: &AppState, wallet_id: Option<i64>) -> Result<HistoryReport> {
    let _running = state.history.lock().await;
    let keys = keys(state)?;
    let wallets = {
        let db = state.db();
        match wallet_id {
            Some(id) => vec![repo::get_wallet(&db, id)?],
            None => repo::list_wallets(&db)?,
        }
    };

    let mut report = HistoryReport::default();
    let mut touched: Vec<String> = Vec::new();
    for wallet in &wallets {
        let owners = store::owner_addresses(&state.db(), wallet.id, &wallet.address)?;
        let (sources, missing) = adapters(wallet.family, &state.http, &keys);
        report
            .missing
            .extend(missing.into_iter().map(|(chain, reason)| MissingSource {
                wallet_id: wallet.id,
                chain,
                reason: match reason {
                    Unavailable::NeedsEtherscanKey => "etherscanKey",
                },
            }));

        for source in sources {
            let chain = source.chain();
            match sync_chain(state, wallet.id, chain, &owners, source.as_ref()).await {
                Ok((ids, backfilling)) => {
                    report.new_transactions += ids.len();
                    report.backfilling |= backfilling;
                    touched.extend(ids);
                }
                Err(err) => report.issues.push(HistoryIssue {
                    wallet_id: wallet.id,
                    chain,
                    message: err.to_string(),
                }),
            }
        }
    }

    touched.sort();
    touched.dedup();
    store::reclassify(&mut state.db(), &touched)?;
    if let Err(err) = backfill_daily_prices(state).await {
        // Values are optional; the history itself is stored already.
        eprintln!("[history] price backfill failed: {err}");
    }
    Ok(report)
}

/// Returns the ids stored and whether older history remains.
async fn sync_chain(
    state: &AppState,
    wallet_id: i64,
    chain: Chain,
    owners: &[String],
    source: &dyn super::HistoryAdapter,
) -> Result<(Vec<String>, bool)> {
    let cursor = store::get_cursor(&state.db(), wallet_id, chain)?.unwrap_or_default();
    let known = store::known_hashes(&state.db(), wallet_id, chain)?;
    let mut fetched: Vec<RawTx> = Vec::new();
    let mut newest: Option<String> = None;

    // Newest first, until we meet history we already have.
    let mut next: Option<String> = None;
    let mut reached_known = false;
    let limit = if cursor.newest.is_none() {
        FIRST_PAGES
    } else {
        NEW_PAGES
    };
    for i in 0..limit {
        let Page { txs, next: after } = source.page(owners, next.as_deref()).await?;
        if i == 0 {
            newest = txs.first().map(|t| t.hash.clone());
        }
        reached_known = txs.iter().any(|t| known.contains(&t.hash));
        fetched.extend(txs);
        next = after;
        if reached_known || next.is_none() {
            break;
        }
    }

    // First sync: whatever is left after the newest pages is the backfill.
    let mut backfill = if cursor.newest.is_none() && !reached_known {
        next.clone()
    } else {
        cursor.backfill.clone()
    };
    let mut complete = cursor.complete || (cursor.newest.is_none() && next.is_none());

    if !complete {
        for _ in 0..BACKFILL_PAGES {
            let Some(from) = backfill.clone() else {
                complete = true;
                break;
            };
            let Page { txs, next: after } = source.page(owners, Some(&from)).await?;
            fetched.extend(txs);
            backfill = after;
            if backfill.is_none() {
                complete = true;
                break;
            }
        }
    }

    let ids = store::insert_txs(
        &mut state.db(),
        wallet_id,
        chain,
        &super::merge_by_hash(fetched),
    )?;
    store::set_cursor(
        &state.db(),
        wallet_id,
        chain,
        &Cursor {
            newest: newest.or(cursor.newest),
            backfill: if complete { None } else { backfill },
            complete,
        },
    )?;
    Ok((ids, !complete))
}

/// Caches daily prices for every asset seen in transactions, so their values
/// at the time can be shown and used for cost basis.
async fn backfill_daily_prices(state: &AppState) -> Result<()> {
    let now = crate::db::now_ms();
    let since = now - MAX_DAYS * DAY_MS;
    let needs = store::assets_needing_prices(&state.db(), since)?;
    if needs.is_empty() {
        return Ok(());
    }
    let today = now.div_euclid(DAY_MS);
    let oldest = needs
        .iter()
        .map(|(_, ts)| *ts)
        .min()
        .unwrap_or(now)
        .max(since);
    let start_day = oldest.div_euclid(DAY_MS);
    let keys: Vec<String> = needs.into_iter().map(|(k, _)| k).collect();
    let cached = store::daily_prices(&state.db(), &keys, start_day, today)?;
    let wanted = (today - start_day + 1) as usize;
    let missing: Vec<String> = keys
        .into_iter()
        .filter(|k| cached.get(k).is_none_or(|s| s.len() + 2 < wanted))
        .collect();
    if missing.is_empty() {
        return Ok(());
    }
    let fetched = DefiLlama::new(state.http.clone())
        .chart(&missing, start_day * 86_400, wanted as u32, 86_400)
        .await?;
    let daily: HashMap<String, Vec<(i64, Decimal)>> = fetched
        .into_iter()
        .map(|(key, series)| {
            let days = series
                .into_iter()
                .map(|(secs, p)| (secs.div_euclid(86_400), p))
                .collect();
            (key, days)
        })
        .collect();
    store::store_daily_prices(&mut state.db(), &daily)
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HistoryPoint {
    pub t: i64,
    pub value_usd: Decimal,
}

/// Portfolio value between `from` and `to` (Unix ms), optionally restricted to
/// some assets. Sub-daily ranges use hourly prices fetched on demand; longer
/// ones use (and fill) the daily cache.
pub async fn portfolio_history(
    state: &AppState,
    from: i64,
    to: i64,
    asset_ids: Option<Vec<String>>,
) -> Result<Vec<HistoryPoint>> {
    let now = crate::db::now_ms();
    let to = to.min(now);
    let from = from.max(now - MAX_DAYS * DAY_MS).min(to);
    let step = replay::step_for(to - from);
    let points = replay::sample_points(from, to, step);

    let (holdings, legs) = {
        let db = state.db();
        let holdings = store::current_holdings(&db, asset_ids.as_deref())?;
        let legs = store::legs_since(&db, from, asset_ids.as_deref())?;
        (holdings, legs)
    };
    let now_amounts: HashMap<String, Decimal> = holdings
        .iter()
        .map(|(a, _, amount, _)| (a.clone(), *amount))
        .collect();
    let current_prices: HashMap<String, Decimal> = holdings
        .iter()
        .map(|(_, k, _, price)| (k.clone(), *price))
        .collect();
    let mut asset_keys: HashMap<String, String> = holdings
        .iter()
        .map(|(a, k, _, _)| (a.clone(), k.clone()))
        .collect();
    let leg_assets: Vec<String> = legs
        .iter()
        .map(|(_, a, _)| a.clone())
        .filter(|a| !asset_keys.contains_key(a))
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    asset_keys.extend(store::price_keys_for(&state.db(), &leg_assets)?);

    let snapshots = replay::holdings_at(&now_amounts, &legs, &points);
    let mut keys: Vec<String> = snapshots
        .iter()
        .flat_map(|s| s.keys())
        .filter_map(|a| asset_keys.get(a).cloned())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    keys.sort();

    let series = price_series(state, &keys, from, to, step).await;
    let values = replay::value_series(&points, &snapshots, &asset_keys, &series, &current_prices);
    Ok(points
        .into_iter()
        .zip(values)
        .map(|(t, value_usd)| HistoryPoint { t, value_usd })
        .collect())
}

/// Price series in ms timestamps. Falls back to whatever is cached when the
/// network is unavailable.
async fn price_series(
    state: &AppState,
    keys: &[String],
    from: i64,
    to: i64,
    step: i64,
) -> HashMap<String, Vec<(i64, Decimal)>> {
    if keys.is_empty() {
        return HashMap::new();
    }
    let from_day = from.div_euclid(DAY_MS);
    let to_day = to.div_euclid(DAY_MS);
    let cached = store::daily_prices(&state.db(), keys, from_day, to_day).unwrap_or_default();
    let as_ms = |m: HashMap<String, Vec<(i64, Decimal)>>| {
        m.into_iter()
            .map(|(k, s)| (k, s.into_iter().map(|(d, p)| (d * DAY_MS, p)).collect()))
            .collect::<HashMap<_, _>>()
    };

    let llama = DefiLlama::new(state.http.clone());
    if step >= DAY_MS {
        let wanted = (to_day - from_day + 1) as usize;
        let missing: Vec<String> = keys
            .iter()
            .filter(|k| cached.get(*k).is_none_or(|s| s.len() + 2 < wanted))
            .cloned()
            .collect();
        if !missing.is_empty() {
            if let Ok(fetched) = llama
                .chart(&missing, from_day * 86_400, wanted as u32, 86_400)
                .await
            {
                let daily: HashMap<String, Vec<(i64, Decimal)>> = fetched
                    .into_iter()
                    .map(|(k, s)| {
                        (
                            k,
                            s.into_iter()
                                .map(|(secs, p)| (secs.div_euclid(86_400), p))
                                .collect(),
                        )
                    })
                    .collect();
                let _ = store::store_daily_prices(&mut state.db(), &daily);
                return as_ms(
                    store::daily_prices(&state.db(), keys, from_day, to_day).unwrap_or_default(),
                );
            }
        }
        return as_ms(cached);
    }

    let points = ((to - from) / step + 2) as u32;
    match llama.chart(keys, from / 1000, points, step / 1000).await {
        Ok(fetched) => fetched
            .into_iter()
            .map(|(k, s)| (k, s.into_iter().map(|(secs, p)| (secs * 1000, p)).collect()))
            .collect(),
        Err(_) => as_ms(cached),
    }
}
