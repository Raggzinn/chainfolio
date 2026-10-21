//! TRON: TRX and TRC-20 balances from TronGrid's keyless API. Token metadata
//! comes from constant contract calls; set `TRONGRID_API_KEY` to send a
//! `TRON-PRO-API-KEY` header and get higher rate limits.

use std::collections::HashMap;

use async_trait::async_trait;
use bitcoin::base58;
use rust_decimal::Decimal;
use serde::Deserialize;
use serde_json::json;

use super::{AssetMeta, Chain, ChainAdapter, Holding, from_base_units};
use crate::error::{Error, Result};

const API: &str = "https://api.trongrid.io";
/// Unknown TRC-20 contracts looked up per refresh. Busy addresses collect
/// hundreds of airdropped spam tokens; each lookup costs two requests.
const MAX_UNKNOWN_TOKENS: usize = 25;

/// TRC-20 tokens whose metadata is known, so they need no lookup.
const KNOWN_TOKENS: &[(&str, &str, &str, u32)] = &[(
    "TR7NHqjeKQxGTCi8q8ZY4pL8otSzgjLj6t",
    "USDT",
    "Tether USD",
    6,
)];

/// Mainnet addresses are base58check of `0x41 ‖ 20-byte account id`.
pub fn is_tron_address(input: &str) -> bool {
    input.len() == 34
        && input.starts_with('T')
        && base58::decode_check(input).is_ok_and(|bytes| bytes.len() == 21 && bytes[0] == 0x41)
}

pub struct TronAdapter {
    http: reqwest::Client,
    api_key: Option<String>,
}

impl TronAdapter {
    pub fn new(http: reqwest::Client) -> Self {
        Self {
            http,
            api_key: std::env::var("TRONGRID_API_KEY")
                .ok()
                .filter(|k| !k.is_empty()),
        }
    }

    fn with_key(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match &self.api_key {
            Some(key) => request.header("TRON-PRO-API-KEY", key),
            None => request,
        }
    }

    async fn account(&self, address: &str) -> Result<Option<Account>> {
        let response: AccountsResponse = self
            .with_key(self.http.get(format!("{API}/v1/accounts/{address}")))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        if !response.success {
            return Err(Error::provider("TronGrid returned an error"));
        }
        // Accounts that never received TRX are not activated and come back empty.
        Ok(response.data.into_iter().next())
    }

    /// Calls a view function and returns its ABI-encoded result as hex.
    async fn call(&self, owner: &str, contract: &str, selector: &str) -> Result<String> {
        let response: ConstantResult = self
            .with_key(
                self.http
                    .post(format!("{API}/wallet/triggerconstantcontract")),
            )
            .json(&json!({
                "owner_address": owner,
                "contract_address": contract,
                "function_selector": selector,
                "visible": true,
            }))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        response
            .constant_result
            .into_iter()
            .next()
            .filter(|hex| !hex.is_empty())
            .ok_or_else(|| Error::provider(format!("{selector} reverted on {contract}")))
    }

    async fn token_meta(&self, owner: &str, contract: &str) -> Option<AssetMeta> {
        if let Some((_, symbol, name, decimals)) = KNOWN_TOKENS.iter().find(|t| t.0 == contract) {
            return Some(
                AssetMeta::token(Chain::Tron, contract, *decimals)
                    .with_names(Some((*symbol).into()), Some((*name).into())),
            );
        }
        let decimals = abi_u32(&self.call(owner, contract, "decimals()").await.ok()?)?;
        let symbol = self
            .call(owner, contract, "symbol()")
            .await
            .ok()
            .and_then(|hex| abi_string(&hex));
        Some(AssetMeta::token(Chain::Tron, contract, decimals).with_names(symbol, None))
    }
}

#[async_trait]
impl ChainAdapter for TronAdapter {
    fn chain(&self) -> Chain {
        Chain::Tron
    }

    async fn balances(&self, address: &str) -> Result<Vec<Holding>> {
        let Some(account) = self.account(address).await? else {
            return Ok(Vec::new());
        };
        let mut holdings = Vec::new();
        if let Some(sun) = account.balance.filter(|b| *b > 0) {
            holdings.push(Holding {
                asset: AssetMeta::native(Chain::Tron, "TRX", "TRON", 6, "tron"),
                amount: Decimal::from_i128_with_scale(i128::from(sun), 6).normalize(),
                locked: None,
            });
        }

        let mut unknown = 0;
        for (contract, raw) in trc20_balances(&account) {
            let known = KNOWN_TOKENS.iter().any(|t| t.0 == contract);
            if !known {
                if unknown == MAX_UNKNOWN_TOKENS {
                    continue;
                }
                unknown += 1;
            }
            let Some(asset) = self.token_meta(address, &contract).await else {
                continue;
            };
            if let Some(amount) = from_base_units(&raw, asset.decimals).filter(|a| !a.is_zero()) {
                holdings.push(Holding {
                    asset,
                    amount,
                    locked: None,
                });
            }
        }
        Ok(holdings)
    }
}

#[derive(Deserialize)]
struct AccountsResponse {
    #[serde(default)]
    data: Vec<Account>,
    #[serde(default)]
    success: bool,
}

#[derive(Deserialize)]
struct Account {
    /// TRX in sun (1e-6). Absent when zero.
    balance: Option<u64>,
    /// One single-entry map `{contract: raw_amount}` per token.
    #[serde(default)]
    trc20: Vec<HashMap<String, String>>,
}

#[derive(Deserialize)]
struct ConstantResult {
    #[serde(default)]
    constant_result: Vec<String>,
}

/// Non-zero TRC-20 balances, known tokens first so spam can't crowd them out.
fn trc20_balances(account: &Account) -> Vec<(String, String)> {
    let mut balances: Vec<(String, String)> = account
        .trc20
        .iter()
        .flatten()
        .filter(|(_, raw)| !raw.trim_start_matches('0').is_empty())
        .map(|(c, raw)| (c.clone(), raw.clone()))
        .collect();
    balances.sort_by_key(|(c, _)| !KNOWN_TOKENS.iter().any(|t| t.0 == c));
    balances
}

/// Decodes a `uint` return value that fits in u32 (e.g. `decimals()`).
fn abi_u32(hex: &str) -> Option<u32> {
    let word = hex.get(hex.len().checked_sub(64)?..)?;
    u32::from_str_radix(word.trim_start_matches('0'), 16)
        .ok()
        .or_else(|| word.bytes().all(|b| b == b'0').then_some(0))
}

/// Decodes an ABI `string` return value, or a `bytes32` one as older tokens use.
fn abi_string(hex: &str) -> Option<String> {
    let bytes = hex::decode(hex).ok()?;
    let raw = if bytes.len() == 32 {
        bytes
    } else {
        let len = usize::try_from(u64::from_be_bytes(bytes.get(56..64)?.try_into().ok()?)).ok()?;
        bytes.get(64..64 + len)?.to_vec()
    };
    let text = String::from_utf8(raw).ok()?;
    let text = text.trim_end_matches('\0').trim();
    (!text.is_empty() && text.len() <= 32).then(|| text.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_addresses() {
        assert!(is_tron_address("TR7NHqjeKQxGTCi8q8ZY4pL8otSzgjLj6t"));
        assert!(is_tron_address("TUH23i3QneKAb4xTSsw9QF8hp197T3Zpnt"));
        // Bad checksum, wrong length, a Bitcoin address.
        assert!(!is_tron_address("TR7NHqjeKQxGTCi8q8ZY4pL8otSzgjLj6u"));
        assert!(!is_tron_address("TR7NHqjeKQxGTCi8q8ZY4pL8otSzgjLj6"));
        assert!(!is_tron_address("1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa"));
    }

    #[test]
    fn decodes_abi_values() {
        let usdt = "0000000000000000000000000000000000000000000000000000000000000020\
                    0000000000000000000000000000000000000000000000000000000000000004\
                    5553445400000000000000000000000000000000000000000000000000000000";
        assert_eq!(abi_string(usdt).as_deref(), Some("USDT"));
        let bytes32 = format!("{:0<64}", hex::encode("MKR"));
        assert_eq!(abi_string(&bytes32).as_deref(), Some("MKR"));
        assert_eq!(
            abi_u32("0000000000000000000000000000000000000000000000000000000000000006"),
            Some(6)
        );
        assert_eq!(abi_u32(&"0".repeat(64)), Some(0));
        assert_eq!(abi_u32("06"), None);
    }

    #[test]
    fn parses_trongrid_accounts() {
        let body = r#"{"data": [{"address": "x", "balance": 68807921, "trc20": [
            {"TF6f9yCXxGSpySRGv59eTyzWGVtKCd8oa7": "292292"},
            {"TR7NHqjeKQxGTCi8q8ZY4pL8otSzgjLj6t": "11758007686"},
            {"TE8VXWNoBYm98e8aLQeighNNimNxZNJeQj": "0"}
          ]}], "success": true, "meta": {}}"#;
        let response: AccountsResponse = serde_json::from_str(body).unwrap();
        let account = &response.data[0];
        assert_eq!(account.balance, Some(68_807_921));
        let balances = trc20_balances(account);
        assert_eq!(balances.len(), 2);
        assert_eq!(balances[0].0, "TR7NHqjeKQxGTCi8q8ZY4pL8otSzgjLj6t");

        let empty: AccountsResponse =
            serde_json::from_str(r#"{"data": [], "success": true}"#).unwrap();
        assert!(empty.data.is_empty());
    }
}
