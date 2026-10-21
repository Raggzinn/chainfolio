//! XRP Ledger history via `account_tx`: payments in and out (XRP or issued
//! tokens, using the delivered amount) and the fee the account paid.

use async_trait::async_trait;
use rust_decimal::Decimal;
use serde_json::{Value, json};

use super::{HistoryAdapter, Page, RawTx};
use crate::{
    chains::{
        Chain,
        xrpl::{XrplAdapter, parse_amount, token_asset, xrp_asset},
    },
    error::{Error, Result},
};

const PAGE_SIZE: u32 = 100;
/// Seconds between the Unix epoch and the Ripple epoch (2000-01-01).
const RIPPLE_EPOCH: i64 = 946_684_800;

pub struct XrplHistory {
    rpc: XrplAdapter,
}

impl XrplHistory {
    pub fn new(http: reqwest::Client) -> Self {
        Self {
            rpc: XrplAdapter::new(http),
        }
    }
}

fn drops(value: &str) -> Option<Decimal> {
    value
        .parse::<i64>()
        .ok()
        .map(|d| Decimal::new(d, 6).normalize())
}

/// One `account_tx` entry seen from `owner`. Handles both the v1 (`tx`) and
/// v2 (`tx_json` + `hash`) response shapes.
fn to_raw(entry: &Value, owner: &str) -> Option<RawTx> {
    let tx = entry.get("tx_json").or_else(|| entry.get("tx"))?;
    let hash = entry["hash"].as_str().or_else(|| tx["hash"].as_str())?;
    let meta = &entry["meta"];
    let time = (tx["date"].as_i64().unwrap_or(0) + RIPPLE_EPOCH) * 1000;
    let mut raw = RawTx::new(hash, time);
    raw.failed = meta["TransactionResult"]
        .as_str()
        .is_some_and(|r| r != "tesSUCCESS");

    let sender = tx["Account"].as_str().unwrap_or_default();
    if sender == owner {
        if let Some(fee) = tx["Fee"].as_str().and_then(drops).filter(|f| !f.is_zero()) {
            raw.fee = Some((xrp_asset(), fee));
        }
    }

    if tx["TransactionType"].as_str() == Some("Payment") && !raw.failed {
        let destination = tx["Destination"].as_str().unwrap_or_default();
        let delivered = meta
            .get("delivered_amount")
            .or_else(|| meta.get("DeliveredAmount"))
            .or_else(|| tx.get("DeliverMax"))
            .or_else(|| tx.get("Amount"))?;
        let (asset, amount) = match delivered {
            Value::String(d) => (xrp_asset(), drops(d)?),
            Value::Object(o) => (
                token_asset(o.get("currency")?.as_str()?, o.get("issuer")?.as_str()?),
                parse_amount(o.get("value")?.as_str()?)?,
            ),
            _ => return Some(raw),
        };
        if sender == owner && destination != owner {
            raw.add(asset, -amount, Some(destination.to_string()));
        } else if destination == owner && sender != owner {
            raw.add(asset, amount, Some(sender.to_string()));
        }
    }
    Some(raw.tidy())
}

#[async_trait]
impl HistoryAdapter for XrplHistory {
    fn chain(&self) -> Chain {
        Chain::Xrp
    }

    async fn page(&self, owners: &[String], cursor: Option<&str>) -> Result<Page> {
        let owner = owners
            .first()
            .ok_or_else(|| Error::provider("no address"))?;
        let mut params = json!({ "account": owner, "limit": PAGE_SIZE, "forward": false });
        if let Some(marker) = cursor {
            params["marker"] =
                serde_json::from_str(marker).map_err(|e| Error::provider(e.to_string()))?;
        }
        let result = self.rpc.request("account_tx", params).await?;
        let txs = result["transactions"]
            .as_array()
            .map(|entries| entries.iter().filter_map(|e| to_raw(e, owner)).collect())
            .unwrap_or_default();
        let next = result
            .get("marker")
            .filter(|m| !m.is_null())
            .map(Value::to_string);
        Ok(Page { txs, next })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const OWNER: &str = "rOwner";

    #[test]
    fn xrp_payment_sent_with_fee() {
        let entry = json!({
            "meta": { "TransactionResult": "tesSUCCESS", "delivered_amount": "25000000" },
            "tx": { "hash": "H1", "Account": OWNER, "Destination": "rDest", "TransactionType": "Payment", "Fee": "12", "date": 0 }
        });
        let raw = to_raw(&entry, OWNER).unwrap();
        assert_eq!(raw.timestamp, RIPPLE_EPOCH * 1000);
        assert_eq!(raw.deltas[0].amount, Decimal::from(-25));
        assert_eq!(raw.fee.unwrap().1, Decimal::new(12, 6));
    }

    #[test]
    fn issued_token_received_v2_shape() {
        let entry = json!({
            "hash": "H2",
            "meta": { "TransactionResult": "tesSUCCESS", "delivered_amount": { "currency": "USD", "issuer": "rIssuer", "value": "10.5" } },
            "tx_json": { "Account": "rSender", "Destination": OWNER, "TransactionType": "Payment", "Fee": "10", "date": 1 }
        });
        let raw = to_raw(&entry, OWNER).unwrap();
        assert!(raw.fee.is_none());
        assert_eq!(raw.deltas[0].amount, Decimal::new(105, 1));
        assert_eq!(raw.deltas[0].asset.contract.as_deref(), Some("USD.rIssuer"));
    }
}
