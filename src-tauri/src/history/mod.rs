//! Transaction history: each adapter pages through an address's past
//! transactions (newest first) and reports what every transaction did to the
//! owner's balances.

pub mod bitcoin;
pub mod classify;
pub mod evm;
pub mod solana;
pub mod sync;
pub mod tron;
pub mod xrpl;

use std::collections::BTreeMap;

use async_trait::async_trait;
use rust_decimal::Decimal;

use crate::{
    chains::{AssetMeta, Chain, Family},
    error::Result,
};

/// Balance change of one asset caused by a transaction.
#[derive(Clone, Debug, PartialEq)]
pub struct Delta {
    pub asset: AssetMeta,
    /// Positive when the owner received, negative when it sent. Excludes fees.
    pub amount: Decimal,
    pub counterparty: Option<String>,
}

/// A transaction as seen from the owner's addresses.
#[derive(Clone, Debug, PartialEq)]
pub struct RawTx {
    pub hash: String,
    /// Unix time in milliseconds.
    pub timestamp: i64,
    pub failed: bool,
    pub deltas: Vec<Delta>,
    /// Network fee, only when the owner paid it.
    pub fee: Option<(AssetMeta, Decimal)>,
}

impl RawTx {
    pub fn new(hash: impl Into<String>, timestamp: i64) -> Self {
        Self {
            hash: hash.into(),
            timestamp,
            failed: false,
            deltas: Vec::new(),
            fee: None,
        }
    }

    /// Adds `amount` of `asset`, folding repeated transfers of the same asset.
    pub fn add(&mut self, asset: AssetMeta, amount: Decimal, counterparty: Option<String>) {
        if amount.is_zero() {
            return;
        }
        match self.deltas.iter_mut().find(|d| d.asset.id() == asset.id()) {
            Some(delta) => {
                delta.amount += amount;
                if delta.counterparty.is_none() {
                    delta.counterparty = counterparty;
                }
            }
            None => self.deltas.push(Delta {
                asset,
                amount,
                counterparty,
            }),
        }
    }

    /// Drops deltas that cancelled out (e.g. a token sent and refunded).
    pub fn tidy(mut self) -> Self {
        self.deltas.retain(|d| !d.amount.is_zero());
        self
    }
}

/// One page of history, newest first, and where the next (older) page starts.
#[derive(Debug, Default)]
pub struct Page {
    pub txs: Vec<RawTx>,
    pub next: Option<String>,
}

/// Merges transactions from several streams (e.g. native and token
/// transfers) that share hashes, keeping newest first.
pub fn merge_by_hash(txs: impl IntoIterator<Item = RawTx>) -> Vec<RawTx> {
    let mut by_hash: BTreeMap<String, RawTx> = BTreeMap::new();
    for tx in txs {
        match by_hash.get_mut(&tx.hash) {
            Some(existing) => {
                for delta in tx.deltas {
                    existing.add(delta.asset, delta.amount, delta.counterparty);
                }
                if existing.fee.is_none() {
                    existing.fee = tx.fee;
                }
                existing.failed |= tx.failed;
                existing.timestamp = existing.timestamp.max(tx.timestamp);
            }
            None => {
                by_hash.insert(tx.hash.clone(), tx);
            }
        }
    }
    let mut out: Vec<RawTx> = by_hash.into_values().map(RawTx::tidy).collect();
    out.sort_by_key(|t| std::cmp::Reverse(t.timestamp));
    out
}

/// Pages through the history of the owner's addresses on one chain.
#[async_trait]
pub trait HistoryAdapter: Send + Sync {
    fn chain(&self) -> Chain;

    /// `cursor` is `None` for the newest page, otherwise a value previously
    /// returned in [`Page::next`].
    async fn page(&self, owners: &[String], cursor: Option<&str>) -> Result<Page>;
}

/// Optional provider keys that unlock history on chains without a free index.
#[derive(Clone, Debug, Default)]
pub struct ApiKeys {
    pub etherscan: Option<String>,
}

/// Adapters available for a family, and the chains left out with the reason.
pub type Adapters = (Vec<Box<dyn HistoryAdapter>>, Vec<(Chain, Unavailable)>);

/// Why a chain has no history adapter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unavailable {
    NeedsEtherscanKey,
}

/// History adapters for every chain of `family`. Chains that can't be read
/// without a key come back in the second list.
pub fn adapters(family: Family, http: &reqwest::Client, keys: &ApiKeys) -> Adapters {
    let mut ok: Vec<Box<dyn HistoryAdapter>> = Vec::new();
    let mut missing = Vec::new();
    for &chain in family.chains() {
        match family {
            Family::Evm => match evm::source(chain, keys) {
                Some(source) => {
                    ok.push(Box::new(evm::EvmHistory::new(chain, source, http.clone())))
                }
                None => missing.push((chain, Unavailable::NeedsEtherscanKey)),
            },
            Family::Solana => ok.push(Box::new(solana::SolanaHistory::new(http.clone()))),
            Family::Bitcoin => ok.push(Box::new(bitcoin::BitcoinHistory::new(http.clone()))),
            Family::Tron => ok.push(Box::new(tron::TronHistory::new(http.clone()))),
            Family::Xrpl => ok.push(Box::new(xrpl::XrplHistory::new(http.clone()))),
        }
    }
    (ok, missing)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eth() -> AssetMeta {
        AssetMeta::native(Chain::Ethereum, "ETH", "Ether", 18, "ethereum")
    }

    #[test]
    fn folds_repeated_assets_and_drops_zeroes() {
        let mut tx = RawTx::new("0x1", 1);
        tx.add(eth(), Decimal::ONE, None);
        tx.add(eth(), Decimal::NEGATIVE_ONE, Some("0xb".into()));
        tx.add(eth(), Decimal::ZERO, None);
        let tx = tx.tidy();
        assert!(tx.deltas.is_empty());
    }

    #[test]
    fn merges_streams_by_hash_newest_first() {
        let mut a = RawTx::new("0xa", 10);
        a.add(eth(), Decimal::ONE, None);
        let mut a2 = RawTx::new("0xa", 10);
        a2.fee = Some((eth(), Decimal::new(1, 3)));
        let b = RawTx::new("0xb", 20);
        let merged = merge_by_hash([a, b, a2]);
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].hash, "0xb");
        assert_eq!(merged[1].deltas.len(), 1);
        assert!(merged[1].fee.is_some());
    }
}
