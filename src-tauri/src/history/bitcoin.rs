//! Bitcoin history from mempool.space (Esplora). Extended keys pass every
//! derived address in use as owners; each transaction is netted against the
//! whole set so change outputs don't show up as receipts.

use std::collections::{BTreeMap, HashSet};

use async_trait::async_trait;
use rust_decimal::Decimal;
use serde::Deserialize;

use super::{HistoryAdapter, Page, RawTx};
use crate::{
    chains::{
        Chain,
        bitcoin::{APIS, btc_asset},
    },
    error::{Error, Result},
};

pub struct BitcoinHistory {
    http: reqwest::Client,
}

impl BitcoinHistory {
    pub fn new(http: reqwest::Client) -> Self {
        Self { http }
    }
}

#[derive(Debug, Deserialize)]
struct EsploraTx {
    txid: String,
    vin: Vec<Vin>,
    vout: Vec<Vout>,
    fee: Option<u64>,
    status: Status,
}

#[derive(Debug, Deserialize)]
struct Vin {
    prevout: Option<Vout>,
}

#[derive(Debug, Deserialize)]
struct Vout {
    scriptpubkey_address: Option<String>,
    value: u64,
}

#[derive(Debug, Deserialize)]
struct Status {
    confirmed: bool,
    block_time: Option<i64>,
}

fn sats(value: i128) -> Decimal {
    Decimal::from_i128_with_scale(value, 8).normalize()
}

/// Net effect of `tx` on the owner set: what came in minus what went out.
fn to_raw(tx: &EsploraTx, owners: &HashSet<&str>) -> RawTx {
    let ours = |addr: &Option<String>| addr.as_deref().is_some_and(|a| owners.contains(a));
    let spent: i128 = tx
        .vin
        .iter()
        .filter_map(|v| v.prevout.as_ref())
        .filter(|p| ours(&p.scriptpubkey_address))
        .map(|p| p.value as i128)
        .sum();
    let received: i128 = tx
        .vout
        .iter()
        .filter(|o| ours(&o.scriptpubkey_address))
        .map(|o| o.value as i128)
        .sum();
    let time = tx.status.block_time.unwrap_or(0) * 1000;
    let mut raw = RawTx::new(&tx.txid, time);
    let fee = if spent > 0 {
        tx.fee.unwrap_or(0) as i128
    } else {
        0
    };
    if fee > 0 {
        raw.fee = Some((btc_asset(), sats(fee)));
    }
    // The fee is reported separately, so the transfer itself excludes it.
    raw.add(btc_asset(), sats(received - spent + fee), None);
    raw.tidy()
}

impl BitcoinHistory {
    async fn get(&self, path: &str) -> Result<Vec<EsploraTx>> {
        let mut last = Error::provider("no Bitcoin API reachable");
        for api in APIS {
            let response = self.http.get(format!("{api}{path}")).send().await;
            match response.and_then(|r| r.error_for_status()) {
                Ok(r) => return Ok(r.json().await?),
                Err(err) => last = err.into(),
            }
        }
        Err(last)
    }
}

#[async_trait]
impl HistoryAdapter for BitcoinHistory {
    fn chain(&self) -> Chain {
        Chain::Bitcoin
    }

    /// The cursor maps each owner address to the last confirmed txid seen, or
    /// to an empty string once that address is exhausted.
    async fn page(&self, owners: &[String], cursor: Option<&str>) -> Result<Page> {
        let state: BTreeMap<String, String> = match cursor {
            Some(c) => serde_json::from_str(c).map_err(|e| Error::provider(e.to_string()))?,
            None => BTreeMap::new(),
        };
        let owner_set: HashSet<&str> = owners.iter().map(String::as_str).collect();
        let mut seen = HashSet::new();
        let mut txs = Vec::new();
        let mut next = BTreeMap::new();

        for owner in owners {
            let path = match state.get(owner) {
                Some(last) if last.is_empty() => continue,
                Some(last) => format!("/address/{owner}/txs/chain/{last}"),
                None if cursor.is_some() => continue,
                None => format!("/address/{owner}/txs"),
            };
            let page = self.get(&path).await?;
            let last_confirmed = page
                .iter()
                .rev()
                .find(|t| t.status.confirmed)
                .map(|t| t.txid.clone());
            // Esplora returns up to 25 confirmed transactions per page.
            let confirmed = page.iter().filter(|t| t.status.confirmed).count();
            next.insert(
                owner.clone(),
                if confirmed >= 25 {
                    last_confirmed.unwrap_or_default()
                } else {
                    String::new()
                },
            );
            for tx in page.iter().filter(|t| t.status.confirmed) {
                if seen.insert(tx.txid.clone()) {
                    txs.push(to_raw(tx, &owner_set));
                }
            }
        }
        txs.sort_by_key(|t| std::cmp::Reverse(t.timestamp));
        let more = next.values().any(|v| !v.is_empty());
        Ok(Page {
            txs,
            next: more
                .then(|| serde_json::to_string(&next))
                .transpose()
                .map_err(|e| Error::provider(e.to_string()))?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn out(addr: &str, value: u64) -> Vout {
        Vout {
            scriptpubkey_address: Some(addr.into()),
            value,
        }
    }

    #[test]
    fn spend_with_change_counts_only_what_left() {
        let tx = EsploraTx {
            txid: "t".into(),
            vin: vec![Vin {
                prevout: Some(out("mine", 100_000)),
            }],
            vout: vec![out("theirs", 60_000), out("change", 39_000)],
            fee: Some(1_000),
            status: Status {
                confirmed: true,
                block_time: Some(10),
            },
        };
        let owners: HashSet<&str> = ["mine", "change"].into_iter().collect();
        let raw = to_raw(&tx, &owners);
        assert_eq!(raw.deltas[0].amount, sats(-60_000));
        assert_eq!(raw.fee.unwrap().1, sats(1_000));
        assert_eq!(raw.timestamp, 10_000);
    }

    #[test]
    fn receipt_has_no_fee() {
        let tx = EsploraTx {
            txid: "t".into(),
            vin: vec![Vin {
                prevout: Some(out("theirs", 50_000)),
            }],
            vout: vec![out("mine", 40_000)],
            fee: Some(10_000),
            status: Status {
                confirmed: true,
                block_time: Some(1),
            },
        };
        let owners: HashSet<&str> = ["mine"].into_iter().collect();
        let raw = to_raw(&tx, &owners);
        assert!(raw.fee.is_none());
        assert_eq!(raw.deltas[0].amount, sats(40_000));
    }
}
