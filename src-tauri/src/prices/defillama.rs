//! DefiLlama coins API: free, keyless, covers every chain we support.
//! https://defillama.com/docs/api (coins.llama.fi)

use std::{collections::HashMap, str::FromStr};

use async_trait::async_trait;
use futures::{StreamExt, TryStreamExt, stream};
use rust_decimal::Decimal;
use serde::Deserialize;

use super::{PriceProvider, Quote};
use crate::error::Result;

const API: &str = "https://coins.llama.fi";
/// Coins per request; keeps URLs comfortably short.
const CHUNK: usize = 60;
/// Requests in flight at once.
const CONCURRENCY: usize = 4;
/// Prices below this confidence come from thin pools and are mostly spam.
const MIN_CONFIDENCE: f64 = 0.8;

pub struct DefiLlama {
    http: reqwest::Client,
}

impl DefiLlama {
    pub fn new(http: reqwest::Client) -> Self {
        Self { http }
    }

    async fn chunk(&self, keys: Vec<String>) -> Result<HashMap<String, Quote>> {
        let joined = keys.join(",");
        let prices: PricesResponse = self
            .http
            .get(format!("{API}/prices/current/{joined}"))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        // The change endpoint is a nice-to-have; prices still count without it.
        let changes = match self
            .http
            .get(format!("{API}/percentage/{joined}?period=24h"))
            .send()
            .await
            .and_then(reqwest::Response::error_for_status)
        {
            Ok(response) => response.json::<ChangesResponse>().await.unwrap_or_default(),
            Err(_) => ChangesResponse::default(),
        };
        Ok(merge(&keys, prices, changes))
    }
}

#[async_trait]
impl PriceProvider for DefiLlama {
    async fn quotes(&self, keys: &[String]) -> Result<HashMap<String, Quote>> {
        let chunks: Vec<HashMap<String, Quote>> =
            stream::iter(keys.chunks(CHUNK).map(<[_]>::to_vec))
                .map(|chunk| self.chunk(chunk))
                .buffer_unordered(CONCURRENCY)
                .try_collect()
                .await?;
        Ok(chunks.into_iter().flatten().collect())
    }
}

#[derive(Debug, Deserialize)]
struct PricesResponse {
    coins: HashMap<String, Coin>,
}

#[derive(Debug, Deserialize)]
struct Coin {
    price: serde_json::Number,
    symbol: Option<String>,
    confidence: Option<f64>,
}

#[derive(Debug, Default, Deserialize)]
struct ChangesResponse {
    coins: HashMap<String, serde_json::Number>,
}

fn decimal(n: &serde_json::Number) -> Option<Decimal> {
    let s = n.to_string();
    Decimal::from_str(&s)
        .or_else(|_| Decimal::from_scientific(&s))
        .ok()
}

/// DefiLlama lowercases EVM contract addresses in its response, while Solana
/// mints are case-sensitive, so match exactly first and then case-insensitively.
fn lookup<'a, T>(map: &'a HashMap<String, T>, key: &str) -> Option<&'a T> {
    map.get(key).or_else(|| {
        map.iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v)
    })
}

fn merge(
    keys: &[String],
    prices: PricesResponse,
    changes: ChangesResponse,
) -> HashMap<String, Quote> {
    keys.iter()
        .filter_map(|key| {
            let coin = lookup(&prices.coins, key)?;
            if coin.confidence.is_some_and(|c| c < MIN_CONFIDENCE) {
                return None;
            }
            let price_usd = decimal(&coin.price)?;
            let change_24h = lookup(&changes.coins, key)
                .and_then(decimal)
                .map(|pct| pct / Decimal::ONE_HUNDRED);
            Some((
                key.clone(),
                Quote {
                    price_usd,
                    change_24h,
                    symbol: coin.symbol.clone(),
                },
            ))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merges_prices_and_changes() {
        let prices: PricesResponse = serde_json::from_str(
            r#"{"coins": {
              "coingecko:ethereum": {"price": 3412.55, "symbol": "ETH", "timestamp": 1, "confidence": 0.99},
              "ethereum:0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48": {"decimals": 6, "price": 0.999912,
                "symbol": "USDC", "timestamp": 1, "confidence": 0.99},
              "base:0xspam": {"price": 120000, "symbol": "SCAM", "timestamp": 1, "confidence": 0.2},
              "solana:tiny": {"price": 1.2e-7, "symbol": "TINY", "timestamp": 1}
            }}"#,
        )
        .unwrap();
        let changes: ChangesResponse =
            serde_json::from_str(r#"{"coins": {"coingecko:ethereum": -1.25}}"#).unwrap();
        let keys: Vec<String> = [
            "coingecko:ethereum",
            "ethereum:0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
            "base:0xspam",
            "solana:tiny",
            "solana:missing",
        ]
        .map(String::from)
        .to_vec();

        let quotes = merge(&keys, prices, changes);
        assert_eq!(quotes.len(), 3);
        let eth = &quotes["coingecko:ethereum"];
        assert_eq!(eth.price_usd, Decimal::new(341255, 2));
        assert_eq!(eth.change_24h, Some(Decimal::new(-125, 4)));
        assert_eq!(eth.symbol.as_deref(), Some("ETH"));
        assert!(quotes.contains_key("ethereum:0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"));
        assert_eq!(quotes["solana:tiny"].price_usd, Decimal::new(12, 8));
        assert!(!quotes.contains_key("base:0xspam"));
    }
}
