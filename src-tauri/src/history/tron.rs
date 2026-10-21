//! TRON history from TronGrid: TRX transfers (with the fee the owner burned)
//! and TRC-20 transfers, merged by transaction id.

use async_trait::async_trait;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{HistoryAdapter, Page, RawTx, merge_by_hash};
use crate::{
    chains::{AssetMeta, Chain, from_base_units},
    error::{Error, Result},
};

const API: &str = "https://api.trongrid.io";
const PAGE_SIZE: usize = 50;

pub struct TronHistory {
    http: reqwest::Client,
}

impl TronHistory {
    pub fn new(http: reqwest::Client) -> Self {
        Self { http }
    }
}

fn trx() -> AssetMeta {
    AssetMeta::native(Chain::Tron, "TRX", "TRON", 6, "tron")
}

/// Hex form (`41…`) of a base58check TRON address, as TronGrid reports raw
/// contract parameters.
fn to_hex(address: &str) -> Option<String> {
    let bytes = bs58::decode(address).into_vec().ok()?;
    (bytes.len() == 25).then(|| hex::encode(&bytes[..21]))
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Cursor {
    trx: Option<String>,
    trc20: Option<String>,
    /// Set once the first page of each stream was fetched.
    started: bool,
}

#[derive(Debug, Deserialize)]
struct Listing {
    data: Vec<Value>,
    meta: Option<ListingMeta>,
}

#[derive(Debug, Deserialize)]
struct ListingMeta {
    fingerprint: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Trc20Transfer {
    transaction_id: String,
    block_timestamp: i64,
    from: String,
    to: String,
    value: String,
    token_info: TokenInfo,
}

#[derive(Debug, Deserialize)]
struct TokenInfo {
    address: String,
    symbol: Option<String>,
    name: Option<String>,
    decimals: u32,
}

/// TRX movement and fee of a native transaction for `owner_hex`.
fn native_tx(value: &Value, owner_hex: &str) -> Option<RawTx> {
    let id = value["txID"].as_str()?;
    let time = value["block_timestamp"].as_i64().unwrap_or(0);
    let mut raw = RawTx::new(id, time);
    raw.failed = value["ret"][0]["contractRet"]
        .as_str()
        .is_some_and(|r| r != "SUCCESS");
    let contract = &value["raw_data"]["contract"][0];
    let params = &contract["parameter"]["value"];
    let from = params["owner_address"]
        .as_str()
        .unwrap_or_default()
        .to_lowercase();
    if contract["type"].as_str() == Some("TransferContract") && !raw.failed {
        let to = params["to_address"]
            .as_str()
            .unwrap_or_default()
            .to_lowercase();
        let amount = params["amount"]
            .as_i64()
            .and_then(|a| from_base_units(&a.to_string(), 6))
            .unwrap_or_default();
        if from == owner_hex && to != owner_hex {
            raw.add(trx(), -amount, Some(to.clone()));
        } else if to == owner_hex && from != owner_hex {
            raw.add(trx(), amount, Some(from.clone()));
        }
    }
    if from == owner_hex {
        let sun =
            value["net_fee"].as_i64().unwrap_or(0) + value["energy_fee"].as_i64().unwrap_or(0);
        let sun = if sun > 0 {
            sun
        } else {
            value["ret"][0]["fee"].as_i64().unwrap_or(0)
        };
        let fee = from_base_units(&sun.to_string(), 6).unwrap_or_default();
        if !fee.is_zero() {
            raw.fee = Some((trx(), fee));
        }
    }
    Some(raw)
}

fn token_tx(t: Trc20Transfer, owner: &str) -> Option<RawTx> {
    let amount = from_base_units(&t.value, t.token_info.decimals)?;
    let asset = AssetMeta::token(Chain::Tron, t.token_info.address, t.token_info.decimals)
        .with_names(t.token_info.symbol, t.token_info.name);
    let mut raw = RawTx::new(&t.transaction_id, t.block_timestamp);
    let signed = match (t.from == owner, t.to == owner) {
        (true, false) => (-amount, Some(t.to)),
        (false, true) => (amount, Some(t.from)),
        _ => (Decimal::ZERO, None),
    };
    raw.add(asset, signed.0, signed.1);
    Some(raw)
}

impl TronHistory {
    async fn list(&self, path: &str, fingerprint: Option<&str>) -> Result<Listing> {
        let mut url = reqwest::Url::parse(&format!("{API}{path}"))
            .map_err(|e| Error::provider(e.to_string()))?;
        {
            let mut pairs = url.query_pairs_mut();
            pairs.append_pair("limit", &PAGE_SIZE.to_string());
            pairs.append_pair("only_confirmed", "true");
            if let Some(fp) = fingerprint {
                pairs.append_pair("fingerprint", fp);
            }
        }
        let request = self.http.get(url);
        Ok(request.send().await?.error_for_status()?.json().await?)
    }
}

#[async_trait]
impl HistoryAdapter for TronHistory {
    fn chain(&self) -> Chain {
        Chain::Tron
    }

    async fn page(&self, owners: &[String], cursor: Option<&str>) -> Result<Page> {
        let owner = owners
            .first()
            .ok_or_else(|| Error::provider("no address"))?;
        let owner_hex = to_hex(owner).ok_or(Error::InvalidAddress)?;
        let state: Cursor = match cursor {
            Some(c) => serde_json::from_str(c).map_err(|e| Error::provider(e.to_string()))?,
            None => Cursor::default(),
        };
        let mut txs = Vec::new();
        let mut next = Cursor {
            started: true,
            ..Cursor::default()
        };

        if !state.started || state.trx.is_some() {
            let listing = self
                .list(
                    &format!("/v1/accounts/{owner}/transactions"),
                    state.trx.as_deref(),
                )
                .await?;
            next.trx = listing.meta.and_then(|m| m.fingerprint);
            txs.extend(listing.data.iter().filter_map(|v| native_tx(v, &owner_hex)));
        }
        if !state.started || state.trc20.is_some() {
            let listing = self
                .list(
                    &format!("/v1/accounts/{owner}/transactions/trc20"),
                    state.trc20.as_deref(),
                )
                .await?;
            next.trc20 = listing.meta.and_then(|m| m.fingerprint);
            txs.extend(
                listing
                    .data
                    .into_iter()
                    .filter_map(|v| serde_json::from_value::<Trc20Transfer>(v).ok())
                    .filter_map(|t| token_tx(t, owner)),
            );
        }

        let more = next.trx.is_some() || next.trc20.is_some();
        Ok(Page {
            txs: merge_by_hash(txs),
            next: more
                .then(|| serde_json::to_string(&next))
                .transpose()
                .map_err(|e| Error::provider(e.to_string()))?,
        })
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    const OWNER: &str = "TQrY8tryqsYVCYS3MFbtffiPp2ccyn4STm";

    #[test]
    fn converts_base58_to_hex() {
        let hex = to_hex(OWNER).unwrap();
        assert!(hex.starts_with("41"));
        assert_eq!(hex.len(), 42);
    }

    #[test]
    fn native_send_with_fee() {
        let owner_hex = to_hex(OWNER).unwrap();
        let tx = json!({
            "txID": "abc", "block_timestamp": 1000,
            "ret": [{ "contractRet": "SUCCESS", "fee": 1_100_000 }],
            "raw_data": { "contract": [{ "type": "TransferContract", "parameter": { "value": {
                "owner_address": owner_hex, "to_address": "41aaaa", "amount": 5_000_000
            }}}]}
        });
        let raw = native_tx(&tx, &owner_hex).unwrap();
        assert_eq!(raw.deltas[0].amount, Decimal::from(-5));
        assert_eq!(raw.fee.unwrap().1, Decimal::new(11, 1));
    }

    #[test]
    fn trc20_receipt() {
        let t: Trc20Transfer = serde_json::from_value(json!({
            "transaction_id": "x", "block_timestamp": 5, "from": "TSender", "to": OWNER, "value": "2500000",
            "token_info": { "address": "TR7NHqjeKQxGTCi8q8ZY4pL8otSzgjLj6t", "symbol": "USDT", "name": "Tether USD", "decimals": 6 }
        }))
        .unwrap();
        let raw = token_tx(t, OWNER).unwrap();
        assert_eq!(raw.deltas[0].amount, Decimal::new(25, 1));
        assert_eq!(raw.deltas[0].asset.symbol.as_deref(), Some("USDT"));
    }
}
