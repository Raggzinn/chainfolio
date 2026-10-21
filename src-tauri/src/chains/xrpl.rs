//! XRP Ledger: XRP and issued-token (trust line) balances over the public
//! JSON-RPC servers. Accepts classic `r…` addresses and X-addresses.

use std::str::FromStr;

use async_trait::async_trait;
use bitcoin::hashes::{Hash, sha256d};
use rust_decimal::Decimal;
use serde::Deserialize;
use serde_json::{Value, json};

use super::{AssetMeta, Chain, ChainAdapter, Holding};
use crate::error::{Error, Result};

const RPCS: &[&str] = &["https://xrplcluster.com", "https://s1.ripple.com:51234"];
/// Trust line pages fetched at most (400 lines each).
const MAX_PAGES: usize = 5;
/// Current reserves, used if the server doesn't report them (drops).
const DEFAULT_RESERVE_BASE: u64 = 1_000_000;
const DEFAULT_RESERVE_INC: u64 = 200_000;
/// Precision of issued-token amounts on the ledger.
const TOKEN_DECIMALS: u32 = 15;

/// Issued tokens with a known market, keyed by `(issuer, currency code)`.
const KNOWN_TOKENS: &[(&str, &str, &str, &str)] = &[(
    "rMxCKbEDwqr76QuheSUMdEGf4B9xJ8m5De",
    "RLUSD",
    "Ripple USD",
    "ripple-usd",
)];

const CLASSIC_PREFIX: u8 = 0x00;
const X_MAINNET_PREFIX: [u8; 2] = [0x05, 0x44];

fn decode_check(input: &str) -> Option<Vec<u8>> {
    let bytes = bs58::decode(input)
        .with_alphabet(bs58::Alphabet::RIPPLE)
        .into_vec()
        .ok()?;
    let (payload, checksum) = bytes.split_at(bytes.len().checked_sub(4)?);
    let hash = sha256d::Hash::hash(payload);
    (hash.as_byte_array()[..4] == *checksum).then(|| payload.to_vec())
}

fn encode_check(payload: &[u8]) -> String {
    let hash = sha256d::Hash::hash(payload);
    let mut bytes = payload.to_vec();
    bytes.extend_from_slice(&hash.as_byte_array()[..4]);
    bs58::encode(bytes)
        .with_alphabet(bs58::Alphabet::RIPPLE)
        .into_string()
}

/// Classic address: base58check (Ripple alphabet) of `0x00 ‖ 20-byte account id`.
pub fn is_classic_address(input: &str) -> bool {
    (25..=35).contains(&input.len())
        && input.starts_with('r')
        && decode_check(input).is_some_and(|p| p.len() == 21 && p[0] == CLASSIC_PREFIX)
}

/// Decodes a mainnet X-address (XLS-5d) into its classic address and
/// destination tag. Tags only route deposits; balances belong to the account.
pub fn decode_x_address(input: &str) -> Option<(String, Option<u32>)> {
    if !input.starts_with('X') {
        return None;
    }
    let payload = decode_check(input)?;
    if payload.len() != 31 || payload[..2] != X_MAINNET_PREFIX {
        return None;
    }
    let mut classic = vec![CLASSIC_PREFIX];
    classic.extend_from_slice(&payload[2..22]);
    let tag = u32::from_le_bytes(payload[23..27].try_into().ok()?);
    let tag = match payload[22] {
        0 => None,
        1 => Some(tag),
        _ => return None,
    };
    Some((encode_check(&classic), tag))
}

/// Ledger currency codes are 3 ASCII chars or 40 hex chars (often ASCII
/// padded with zeros, like `524C5553…` for RLUSD).
fn currency_symbol(code: &str) -> String {
    if code.len() == 40 && !code.starts_with("00") {
        if let Ok(bytes) = hex::decode(code) {
            let text: String = bytes
                .into_iter()
                .take_while(|b| *b != 0)
                .map(char::from)
                .collect();
            if !text.is_empty() && text.chars().all(|c| c.is_ascii_graphic()) {
                return text;
            }
        }
        return format!("{}…", &code[..6]);
    }
    code.to_string()
}

pub fn parse_amount(value: &str) -> Option<Decimal> {
    Decimal::from_str(value)
        .or_else(|_| Decimal::from_scientific(value))
        .ok()
}

pub struct XrplAdapter {
    http: reqwest::Client,
}

impl XrplAdapter {
    pub fn new(http: reqwest::Client) -> Self {
        Self { http }
    }

    /// rippled JSON-RPC: errors come back inside `result` with `status: error`.
    pub async fn request(&self, method: &str, params: Value) -> Result<Value> {
        let body = json!({ "method": method, "params": [params] });
        let mut last = Error::provider("no XRPL server configured");
        for endpoint in RPCS {
            let response = async {
                let mut value: Value = self
                    .http
                    .post(*endpoint)
                    .json(&body)
                    .send()
                    .await?
                    .error_for_status()?
                    .json()
                    .await?;
                Ok::<_, Error>(value["result"].take())
            };
            match response.await {
                Ok(result) if result["status"] == "error" => {
                    let code = result["error"].as_str().unwrap_or("unknown error");
                    return Err(Error::provider(code.to_string()));
                }
                Ok(result) => return Ok(result),
                Err(err) => last = err,
            }
        }
        Err(last)
    }

    async fn reserves(&self) -> (u64, u64) {
        let state = self.request("server_state", json!({})).await.ok();
        let ledger = state.as_ref().map(|s| &s["state"]["validated_ledger"]);
        let get = |key: &str, default| ledger.and_then(|l| l[key].as_u64()).unwrap_or(default);
        (
            get("reserve_base", DEFAULT_RESERVE_BASE),
            get("reserve_inc", DEFAULT_RESERVE_INC),
        )
    }

    async fn trust_lines(&self, account: &str) -> Result<Vec<TrustLine>> {
        let mut lines = Vec::new();
        let mut marker: Option<Value> = None;
        for _ in 0..MAX_PAGES {
            let mut params =
                json!({ "account": account, "ledger_index": "validated", "limit": 400 });
            if let Some(m) = marker.take() {
                params["marker"] = m;
            }
            let mut page = self.request("account_lines", params).await?;
            let batch: Vec<TrustLine> = serde_json::from_value(page["lines"].take())
                .map_err(|e| Error::provider(format!("bad account_lines: {e}")))?;
            lines.extend(batch);
            match page.get_mut("marker").map(Value::take) {
                Some(m) if !m.is_null() => marker = Some(m),
                _ => break,
            }
        }
        Ok(lines)
    }
}

#[derive(Debug, Deserialize)]
struct TrustLine {
    /// The counterparty; for a holder's line, the token issuer.
    account: String,
    balance: String,
    currency: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct AccountRoot {
    /// XRP in drops (1e-6).
    balance: String,
    #[serde(default)]
    owner_count: u64,
}

/// Holdings from an account root and its trust lines. Part of the XRP balance
/// is locked as reserve: a base amount plus one increment per owned object.
pub fn xrp_asset() -> AssetMeta {
    AssetMeta::native(Chain::Xrp, "XRP", "XRP", 6, "ripple")
}

/// Issued token `currency` from `issuer`, priced through CoinGecko when known.
pub fn token_asset(currency: &str, issuer: &str) -> AssetMeta {
    let symbol = currency_symbol(currency);
    let known = KNOWN_TOKENS
        .iter()
        .find(|(i, code, _, _)| *i == issuer && *code == symbol);
    let mut asset = AssetMeta::token(Chain::Xrp, format!("{currency}.{issuer}"), TOKEN_DECIMALS)
        .with_names(Some(symbol), known.map(|k| k.2.to_string()));
    if let Some((_, _, _, gecko)) = known {
        asset.price_key = format!("coingecko:{gecko}");
    }
    asset
}

fn holdings(root: &AccountRoot, lines: Vec<TrustLine>, reserves: (u64, u64)) -> Vec<Holding> {
    let mut out = Vec::new();
    let drops: u64 = root.balance.parse().unwrap_or(0);
    if drops > 0 {
        let reserve = reserves
            .0
            .saturating_add(reserves.1.saturating_mul(root.owner_count));
        let xrp = |d: u64| Decimal::from_i128_with_scale(i128::from(d), 6).normalize();
        out.push(Holding {
            asset: xrp_asset(),
            amount: xrp(drops),
            locked: Some(xrp(reserve.min(drops))),
        });
    }
    for line in lines {
        // Negative balances are amounts this account owes as the issuer.
        let Some(amount) =
            parse_amount(&line.balance).filter(|a| a.is_sign_positive() && !a.is_zero())
        else {
            continue;
        };
        out.push(Holding {
            asset: token_asset(&line.currency, &line.account),
            amount: amount.normalize(),
            locked: None,
        });
    }
    out
}

#[async_trait]
impl ChainAdapter for XrplAdapter {
    fn chain(&self) -> Chain {
        Chain::Xrp
    }

    async fn balances(&self, address: &str) -> Result<Vec<Holding>> {
        let info = self
            .request(
                "account_info",
                json!({ "account": address, "ledger_index": "validated" }),
            )
            .await;
        let mut info = match info {
            Ok(info) => info,
            // Unfunded accounts don't exist on the ledger yet.
            Err(Error::Provider(code)) if code == "actNotFound" => return Ok(Vec::new()),
            Err(err) => return Err(err),
        };
        let root: AccountRoot = serde_json::from_value(info["account_data"].take())
            .map_err(|e| Error::provider(format!("bad account_info: {e}")))?;
        let lines = self.trust_lines(address).await?;
        Ok(holdings(&root, lines, self.reserves().await))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_classic_addresses() {
        for addr in [
            "rHb9CJAWyB4rj91VRWn96DkukG4bwdtyTh",
            "rMxCKbEDwqr76QuheSUMdEGf4B9xJ8m5De",
            "rrrrrrrrrrrrrrrrrrrrBZbvji",
        ] {
            assert!(is_classic_address(addr), "{addr}");
        }
        for addr in [
            "rHb9CJAWyB4rj91VRWn96DkukG4bwdtyTi",
            "rHb9CJAWyB4rj91VRWn96DkukG4bwdty",
            "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa",
        ] {
            assert!(!is_classic_address(addr), "{addr}");
        }
    }

    #[test]
    fn decodes_x_addresses() {
        let classic = "rMxCKbEDwqr76QuheSUMdEGf4B9xJ8m5De";
        let account = decode_check(classic).unwrap();
        for tag in [None, Some(12345u32)] {
            let mut payload = X_MAINNET_PREFIX.to_vec();
            payload.extend_from_slice(&account[1..]);
            payload.push(u8::from(tag.is_some()));
            payload.extend_from_slice(&tag.unwrap_or(0).to_le_bytes());
            payload.extend_from_slice(&[0; 4]);
            let x = encode_check(&payload);
            assert!(x.starts_with('X'), "{x}");
            assert_eq!(decode_x_address(&x), Some((classic.to_string(), tag)));
        }
        // XLS-5d reference vector (no tag).
        assert_eq!(
            decode_x_address("XVLhHMPHU98es4dbozjVtdWzVrDjtV5fdx1mHp98tDMoQXb"),
            Some(("rGWrZyQqhTp9Xu7G5Pkayo7bXjH4k4QYpf".to_string(), None))
        );
        assert_eq!(decode_x_address(classic), None);
        assert_eq!(
            decode_x_address("XVLhHMPHU98es4dbozjVtdWzVrDjtV5fdx1mHp98tDMoQXc"),
            None
        );
    }

    #[test]
    fn decodes_currency_codes() {
        assert_eq!(currency_symbol("USD"), "USD");
        assert_eq!(
            currency_symbol("524C555344000000000000000000000000000000"),
            "RLUSD"
        );
        assert_eq!(
            currency_symbol("03ABCDEF00000000000000000000000000000000"),
            "03ABCD…"
        );
    }

    #[test]
    fn builds_holdings_with_reserve_and_tokens() {
        let root: AccountRoot =
            serde_json::from_str(r#"{"Account": "r", "Balance": "35223332", "OwnerCount": 14}"#)
                .unwrap();
        let lines: Vec<TrustLine> = serde_json::from_str(
            r#"[
              {"account": "rMxCKbEDwqr76QuheSUMdEGf4B9xJ8m5De", "balance": "12.5",
               "currency": "524C555344000000000000000000000000000000", "limit": "1e9"},
              {"account": "rIssuer", "balance": "-3", "currency": "USD", "limit": "0"},
              {"account": "rOther", "balance": "0", "currency": "EUR", "limit": "10"},
              {"account": "rOther", "balance": "1.5e-3", "currency": "FOO", "limit": "10"}
            ]"#,
        )
        .unwrap();
        let out = holdings(&root, lines, (1_000_000, 200_000));

        assert_eq!(out.len(), 3);
        assert_eq!(out[0].amount, Decimal::from_str("35.223332").unwrap());
        // 1 XRP base + 14 objects × 0.2 XRP.
        assert_eq!(out[0].locked, Some(Decimal::from_str("3.8").unwrap()));
        assert_eq!(out[0].asset.price_key, "coingecko:ripple");
        assert_eq!(out[1].asset.symbol.as_deref(), Some("RLUSD"));
        assert_eq!(out[1].asset.price_key, "coingecko:ripple-usd");
        assert_eq!(out[1].amount, Decimal::new(125, 1));
        assert_eq!(out[2].amount, Decimal::new(15, 4));
        assert!(out[2].asset.price_key.starts_with("xrpl:FOO.rOther"));
    }
}
