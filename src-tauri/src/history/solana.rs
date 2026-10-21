//! Solana history over the public RPC: signatures for the address, then each
//! transaction parsed, with SOL and SPL changes taken from the pre/post
//! balances the runtime records.

use async_trait::async_trait;
use rust_decimal::Decimal;
use serde::Deserialize;
use serde_json::{Value, json};

use super::{HistoryAdapter, Page, RawTx};
use crate::{
    chains::{
        Chain, from_base_units, rpc,
        solana::{SOLANA_RPCS, mint_asset, sol_asset},
    },
    error::{Error, Result},
};

const PAGE_SIZE: usize = 40;

pub struct SolanaHistory {
    http: reqwest::Client,
}

impl SolanaHistory {
    pub fn new(http: reqwest::Client) -> Self {
        Self { http }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Signature {
    signature: String,
    block_time: Option<i64>,
    err: Option<Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ParsedTx {
    block_time: Option<i64>,
    meta: Option<Meta>,
    transaction: Option<TxBody>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Meta {
    err: Option<Value>,
    fee: u64,
    #[serde(default)]
    pre_balances: Vec<u64>,
    #[serde(default)]
    post_balances: Vec<u64>,
    #[serde(default)]
    pre_token_balances: Vec<TokenBalance>,
    #[serde(default)]
    post_token_balances: Vec<TokenBalance>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TokenBalance {
    mint: String,
    owner: Option<String>,
    ui_token_amount: UiAmount,
}

#[derive(Debug, Deserialize)]
struct UiAmount {
    amount: String,
    decimals: u32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TxBody {
    message: Message,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Message {
    account_keys: Vec<AccountKey>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum AccountKey {
    Parsed { pubkey: String },
    Plain(String),
}

impl AccountKey {
    fn key(&self) -> &str {
        match self {
            Self::Parsed { pubkey } => pubkey,
            Self::Plain(key) => key,
        }
    }
}

/// What a parsed transaction did to `owner`.
fn to_raw(signature: &str, fallback_time: Option<i64>, tx: ParsedTx, owner: &str) -> Option<RawTx> {
    let meta = tx.meta?;
    let keys: Vec<String> = tx
        .transaction?
        .message
        .account_keys
        .iter()
        .map(|k| k.key().to_string())
        .collect();
    let time = tx.block_time.or(fallback_time).unwrap_or(0) * 1000;
    let mut raw = RawTx::new(signature, time);
    raw.failed = meta.err.is_some();

    let fee_payer = keys.first().is_some_and(|k| k == owner);
    let fee = if fee_payer {
        from_base_units(&meta.fee.to_string(), 9).unwrap_or_default()
    } else {
        Decimal::ZERO
    };
    if !fee.is_zero() {
        raw.fee = Some((sol_asset(), fee));
    }

    if let Some(index) = keys.iter().position(|k| k == owner) {
        let pre = meta.pre_balances.get(index).copied().unwrap_or(0) as i128;
        let post = meta.post_balances.get(index).copied().unwrap_or(0) as i128;
        let lamports = post - pre;
        let change = Decimal::from_i128_with_scale(lamports, 9) + fee;
        raw.add(sol_asset(), change, None);
    }

    // Token accounts owned by the address; their keys aren't the owner itself.
    let mut token_change: std::collections::BTreeMap<String, (u32, i128)> = Default::default();
    for (sign, balances) in [
        (-1i128, &meta.pre_token_balances),
        (1, &meta.post_token_balances),
    ] {
        for balance in balances
            .iter()
            .filter(|b| b.owner.as_deref() == Some(owner))
        {
            let amount: i128 = balance.ui_token_amount.amount.parse().unwrap_or(0);
            let entry = token_change
                .entry(balance.mint.clone())
                .or_insert((balance.ui_token_amount.decimals, 0));
            entry.1 += sign * amount;
        }
    }
    for (mint, (decimals, raw_amount)) in token_change {
        if raw_amount == 0 || decimals > 28 {
            continue;
        }
        raw.add(
            mint_asset(mint, decimals),
            Decimal::from_i128_with_scale(raw_amount, decimals),
            None,
        );
    }
    Some(raw.tidy())
}

#[async_trait]
impl HistoryAdapter for SolanaHistory {
    fn chain(&self) -> Chain {
        Chain::Solana
    }

    async fn page(&self, owners: &[String], cursor: Option<&str>) -> Result<Page> {
        let owner = owners
            .first()
            .ok_or_else(|| Error::provider("no address"))?;
        let mut options = json!({ "limit": PAGE_SIZE });
        if let Some(before) = cursor {
            options["before"] = json!(before);
        }
        let signatures: Vec<Signature> = rpc::call(
            &self.http,
            SOLANA_RPCS,
            "getSignaturesForAddress",
            json!([owner, options]),
        )
        .await?;

        let params = signatures
            .iter()
            .map(|s| {
                json!([s.signature, { "encoding": "jsonParsed", "maxSupportedTransactionVersion": 0 }])
            })
            .collect();
        let parsed = rpc::batch(&self.http, SOLANA_RPCS, "getTransaction", params).await?;

        let mut txs = Vec::new();
        for (sig, value) in signatures.iter().zip(parsed) {
            let raw = value
                .and_then(|v| serde_json::from_value::<ParsedTx>(v).ok())
                .and_then(|tx| to_raw(&sig.signature, sig.block_time, tx, owner));
            match raw {
                Some(raw) => txs.push(raw),
                // Keep a placeholder so the cursor still advances past it.
                None => {
                    let mut raw = RawTx::new(&sig.signature, sig.block_time.unwrap_or(0) * 1000);
                    raw.failed = sig.err.is_some();
                    txs.push(raw);
                }
            }
        }
        let next = (signatures.len() == PAGE_SIZE)
            .then(|| signatures.last().map(|s| s.signature.clone()))
            .flatten();
        Ok(Page { txs, next })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const OWNER: &str = "Owner1111111111111111111111111111111111111";

    fn tx(json: Value) -> ParsedTx {
        serde_json::from_value(json).unwrap()
    }

    #[test]
    fn sol_sent_by_fee_payer_excludes_the_fee() {
        let parsed = tx(json!({
            "blockTime": 1_700_000_000,
            "meta": { "err": null, "fee": 5000, "preBalances": [2_000_005_000u64, 0], "postBalances": [1_000_000_000u64, 1_000_000_000u64] },
            "transaction": { "message": { "accountKeys": [{ "pubkey": OWNER }, { "pubkey": "Other" }] } }
        }));
        let raw = to_raw("sig", None, parsed, OWNER).unwrap();
        assert_eq!(raw.timestamp, 1_700_000_000_000);
        assert_eq!(raw.deltas.len(), 1);
        assert_eq!(raw.deltas[0].amount, Decimal::NEGATIVE_ONE);
        assert_eq!(raw.fee.unwrap().1, Decimal::new(5, 6));
    }

    #[test]
    fn token_received_by_owned_account() {
        let parsed = tx(json!({
            "meta": {
                "err": null, "fee": 5000, "preBalances": [1, 1], "postBalances": [1, 1],
                "preTokenBalances": [],
                "postTokenBalances": [{ "mint": "Mint", "owner": OWNER, "uiTokenAmount": { "amount": "2500000", "decimals": 6 } }]
            },
            "transaction": { "message": { "accountKeys": ["Payer", "Ata"] } }
        }));
        let raw = to_raw("sig", Some(5), parsed, OWNER).unwrap();
        assert!(raw.fee.is_none());
        assert_eq!(raw.deltas.len(), 1);
        assert_eq!(raw.deltas[0].amount, Decimal::new(25, 1));
        assert_eq!(raw.deltas[0].asset.contract.as_deref(), Some("Mint"));
    }
}
