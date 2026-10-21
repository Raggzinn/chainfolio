//! Refreshes balances and prices for tracked wallets.

use std::collections::BTreeMap;

use futures::future::join_all;
use serde::Serialize;

use crate::{
    chains::{self, Chain, Family, Holding, bitcoin::BitcoinAdapter},
    db::repo::{self, ChainBalances, Portfolio, SyncWrite, Wallet, WalletKind},
    error::Result,
    prices::{PriceProvider, defillama::DefiLlama},
    state::AppState,
};

/// A chain (or the price feed) that failed during a refresh. Balances from a
/// failed chain keep their previous values.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncIssue {
    pub wallet_id: Option<i64>,
    pub chain: Option<Chain>,
    pub message: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshReport {
    pub issues: Vec<SyncIssue>,
    pub portfolio: Portfolio,
}

/// Fetches fresh balances for one wallet (or all of them), reprices every held
/// asset and stores the result.
pub async fn refresh(state: &AppState, wallet_id: Option<i64>) -> Result<RefreshReport> {
    let _running = state.sync.lock().await;
    let wallets = {
        let db = state.db()?;
        match wallet_id {
            Some(id) => vec![repo::get_wallet(&db, id)?],
            None => repo::list_wallets(&db)?,
        }
    };

    let mut write = SyncWrite::default();
    let mut issues = Vec::new();
    for wallet in &wallets {
        for (chain, result) in fetch_wallet(state, wallet, &mut write).await {
            match result {
                Ok(holdings) => {
                    write.synced_wallets.insert(wallet.id);
                    write.balances.push(ChainBalances {
                        wallet_id: wallet.id,
                        chain,
                        holdings: merge(holdings),
                    });
                }
                Err(err) => issues.push(SyncIssue {
                    wallet_id: Some(wallet.id),
                    chain: Some(chain),
                    message: err.to_string(),
                }),
            }
        }
    }

    let mut keys = repo::held_price_keys(&*state.db()?)?;
    keys.extend(
        write
            .balances
            .iter()
            .flat_map(|b| &b.holdings)
            .map(|h| h.asset.price_key.clone()),
    );
    keys.sort();
    keys.dedup();
    match DefiLlama::new(state.http.clone()).quotes(&keys).await {
        Ok(quotes) => write.quotes = quotes,
        Err(err) => issues.push(SyncIssue {
            wallet_id: None,
            chain: None,
            message: format!("prices: {err}"),
        }),
    }

    let mut db = state.db()?;
    repo::save_sync(&mut db, &write)?;
    Ok(RefreshReport {
        issues,
        portfolio: repo::portfolio(&db)?,
    })
}

/// Holdings of `wallet` on every chain of its family, fetched concurrently.
async fn fetch_wallet(
    state: &AppState,
    wallet: &Wallet,
    write: &mut SyncWrite,
) -> Vec<(Chain, Result<Vec<Holding>>)> {
    if wallet.family == Family::Bitcoin && wallet.kind == WalletKind::Xpub {
        let adapter = BitcoinAdapter::new(state.http.clone());
        let result = adapter
            .scan_extended_key(&wallet.address)
            .await
            .map(|(sats, used)| {
                write
                    .used_addresses
                    .extend(used.into_iter().map(|u| (wallet.id, u)));
                chains::bitcoin::btc_holding(sats).into_iter().collect()
            });
        return vec![(Chain::Bitcoin, result)];
    }

    let adapters = chains::adapters(wallet.family, &state.http);
    join_all(
        adapters.iter().map(|adapter| async move {
            (adapter.chain(), adapter.balances(&wallet.address).await)
        }),
    )
    .await
}

/// Sums duplicate entries of the same asset (e.g. two token accounts).
fn merge(holdings: Vec<Holding>) -> Vec<Holding> {
    let mut by_id: BTreeMap<String, Holding> = BTreeMap::new();
    for holding in holdings {
        by_id
            .entry(holding.asset.id())
            .and_modify(|h| h.amount += holding.amount)
            .or_insert(holding);
    }
    by_id.into_values().collect()
}

#[cfg(test)]
mod tests {
    use rust_decimal::Decimal;

    use super::*;
    use crate::chains::AssetMeta;

    #[test]
    fn merges_duplicate_assets() {
        let eth = AssetMeta::native(Chain::Ethereum, "ETH", "Ether", 18, "ethereum");
        let merged = merge(vec![
            Holding {
                asset: eth.clone(),
                amount: Decimal::ONE,
                locked: None,
            },
            Holding {
                asset: eth,
                amount: Decimal::TWO,
                locked: None,
            },
        ]);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].amount, Decimal::from(3));
    }
}
