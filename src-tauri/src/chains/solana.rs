//! Solana: SOL and SPL token balances over the public mainnet RPC. `.sol`
//! names resolve through the keyless SNS SDK proxy.

use std::collections::BTreeMap;

use async_trait::async_trait;
use rust_decimal::Decimal;
use serde::Deserialize;
use serde_json::json;

use super::{AssetMeta, Chain, ChainAdapter, Holding, from_base_units, rpc};
use crate::{
    address,
    error::{Error, Result},
};

const RPCS: &[&str] = &[
    "https://api.mainnet-beta.solana.com",
    "https://solana-rpc.publicnode.com",
];

const TOKEN_PROGRAMS: [&str; 2] = [
    "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
    "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb",
];

/// Names for common mints so they read well before the first price lookup.
const KNOWN_MINTS: &[(&str, &str, &str)] = &[
    (
        "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
        "USDC",
        "USD Coin",
    ),
    (
        "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB",
        "USDT",
        "Tether USD",
    ),
    (
        "mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So",
        "mSOL",
        "Marinade staked SOL",
    ),
    (
        "J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn",
        "JitoSOL",
        "Jito Staked SOL",
    ),
    (
        "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN",
        "JUP",
        "Jupiter",
    ),
    (
        "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263",
        "BONK",
        "Bonk",
    ),
];

pub struct SolanaAdapter {
    http: reqwest::Client,
}

impl SolanaAdapter {
    pub fn new(http: reqwest::Client) -> Self {
        Self { http }
    }
}

#[derive(Deserialize)]
struct Balance {
    value: u64,
}

#[async_trait]
impl ChainAdapter for SolanaAdapter {
    fn chain(&self) -> Chain {
        Chain::Solana
    }

    async fn balances(&self, address: &str) -> Result<Vec<Holding>> {
        let lamports: Balance = rpc::call(&self.http, RPCS, "getBalance", json!([address])).await?;
        let mut holdings = Vec::new();
        if lamports.value > 0 {
            holdings.push(Holding {
                asset: AssetMeta::native(Chain::Solana, "SOL", "Solana", 9, "solana"),
                amount: Decimal::from_i128_with_scale(i128::from(lamports.value), 9).normalize(),
                locked: None,
            });
        }

        let mut accounts = Vec::new();
        for program in TOKEN_PROGRAMS {
            let page: TokenAccounts = rpc::call(
                &self.http,
                RPCS,
                "getTokenAccountsByOwner",
                json!([address, { "programId": program }, { "encoding": "jsonParsed" }]),
            )
            .await?;
            accounts.extend(page.value);
        }
        holdings.extend(parse_token_accounts(accounts));
        Ok(holdings)
    }
}

#[derive(Deserialize)]
struct TokenAccounts {
    value: Vec<TokenAccount>,
}

#[derive(Deserialize)]
struct TokenAccount {
    account: AccountData,
}

#[derive(Deserialize)]
struct AccountData {
    data: ParsedData,
}

#[derive(Deserialize)]
struct ParsedData {
    parsed: Parsed,
}

#[derive(Deserialize)]
struct Parsed {
    info: TokenAccountInfo,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TokenAccountInfo {
    mint: String,
    token_amount: TokenAmount,
}

#[derive(Deserialize)]
struct TokenAmount {
    amount: String,
    decimals: u32,
}

/// Sums token accounts per mint (an owner can hold several accounts of one
/// mint) and drops empty ones.
fn parse_token_accounts(accounts: Vec<TokenAccount>) -> Vec<Holding> {
    let mut by_mint: BTreeMap<String, (u32, Decimal)> = BTreeMap::new();
    for account in accounts {
        let info = account.account.data.parsed.info;
        let decimals = info.token_amount.decimals;
        let Some(amount) = from_base_units(&info.token_amount.amount, decimals) else {
            continue;
        };
        let entry = by_mint
            .entry(info.mint)
            .or_insert((decimals, Decimal::ZERO));
        entry.1 += amount;
    }
    by_mint
        .into_iter()
        .filter(|(_, (_, amount))| !amount.is_zero())
        .map(|(mint, (decimals, amount))| {
            let known = KNOWN_MINTS.iter().find(|(m, _, _)| *m == mint);
            let asset = AssetMeta::token(Chain::Solana, mint, decimals).with_names(
                known.map(|(_, symbol, _)| symbol.to_string()),
                known.map(|(_, _, name)| name.to_string()),
            );
            Holding {
                asset,
                amount,
                locked: None,
            }
        })
        .collect()
}

#[derive(Deserialize)]
struct SnsResponse {
    s: String,
    result: Option<String>,
}

/// Resolves a `.sol` domain to the owner's address.
pub async fn resolve_sns(http: &reqwest::Client, name: &str) -> Result<String> {
    let domain = name.trim_end_matches(".sol");
    let url = format!("https://sdk-proxy.sns.id/resolve/{domain}");
    let response = http.get(url).send().await?;
    if response.status().is_client_error() {
        return Err(Error::NameNotFound(name.to_string()));
    }
    let response: SnsResponse = response.error_for_status()?.json().await?;
    match response.result {
        Some(owner) if response.s == "ok" && address::is_solana_address(&owner) => Ok(owner),
        _ => Err(Error::NameNotFound(name.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sums_token_accounts_per_mint() {
        let body = r#"{"value": [
          {"pubkey": "a", "account": {"data": {"program": "spl-token", "parsed": {"type": "account",
            "info": {"mint": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v", "owner": "x",
              "tokenAmount": {"amount": "1500000", "decimals": 6, "uiAmount": 1.5}}}}}},
          {"pubkey": "b", "account": {"data": {"program": "spl-token", "parsed": {"type": "account",
            "info": {"mint": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v", "owner": "x",
              "tokenAmount": {"amount": "500000", "decimals": 6, "uiAmount": 0.5}}}}}},
          {"pubkey": "c", "account": {"data": {"program": "spl-token", "parsed": {"type": "account",
            "info": {"mint": "So11111111111111111111111111111111111111112", "owner": "x",
              "tokenAmount": {"amount": "0", "decimals": 9, "uiAmount": 0}}}}}}
        ]}"#;
        let accounts: TokenAccounts = serde_json::from_str(body).unwrap();
        let holdings = parse_token_accounts(accounts.value);

        assert_eq!(holdings.len(), 1);
        assert_eq!(holdings[0].amount, Decimal::new(2, 0));
        assert_eq!(holdings[0].asset.symbol.as_deref(), Some("USDC"));
        assert_eq!(
            holdings[0].asset.price_key,
            "solana:EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
        );
    }
}
