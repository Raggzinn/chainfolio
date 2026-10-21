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

/// DefiLlama rejects `/chart` requests asking for more than this many points
/// in total (coins × span).
const CHART_MAX_POINTS: u32 = 500;

#[derive(Debug, Deserialize)]
struct ChartResponse {
    coins: HashMap<String, ChartCoin>,
}

#[derive(Debug, Deserialize)]
struct ChartCoin {
    prices: Vec<ChartPoint>,
}

#[derive(Debug, Deserialize)]
struct ChartPoint {
    timestamp: i64,
    price: serde_json::Number,
}

impl DefiLlama {
    async fn chart_request(&self, url: String) -> Result<ChartResponse> {
        let response = self.http.get(url).send().await?.error_for_status()?;
        Ok(response.json().await?)
    }

    /// Historical USD prices for `keys`: `points` samples every `period_secs`
    /// starting at `start_secs`. Returns `(unix seconds, price)` per key.
    pub async fn chart(
        &self,
        keys: &[String],
        start_secs: i64,
        points: u32,
        period_secs: i64,
    ) -> Result<HashMap<String, Vec<(i64, Decimal)>>> {
        let period = match period_secs {
            s if s >= 86_400 => format!("{}d", s / 86_400),
            s if s >= 3_600 => format!("{}h", s / 3_600),
            s => format!("{}m", (s / 60).max(1)),
        };
        let mut windows = Vec::new();
        let mut start = start_secs;
        let mut left = points;
        while left > 0 {
            let span = left.min(CHART_MAX_POINTS);
            windows.push((start, span));
            start += i64::from(span) * period_secs;
            left -= span;
        }
        let urls: Vec<String> = windows
            .iter()
            .flat_map(|&(start, span)| {
                let per_request = (CHART_MAX_POINTS / span).clamp(1, 25) as usize;
                let period = &period;
                keys.chunks(per_request).map(move |chunk| {
                    let joined = chunk.join(",");
                    format!("{API}/chart/{joined}?start={start}&span={span}&period={period}")
                })
            })
            .collect();
        let responses: Vec<ChartResponse> = stream::iter(urls)
            .map(|url| self.chart_request(url))
            .buffer_unordered(CONCURRENCY)
            .try_collect()
            .await?;
        let mut out: HashMap<String, Vec<(i64, Decimal)>> = HashMap::new();
        for response in responses {
            for (key, coin) in response.coins {
                let series = out.entry(key).or_default();
                series.extend(coin.prices.into_iter().filter_map(|p| {
                    Decimal::from_str(&p.price.to_string())
                        .ok()
                        .map(|price| (p.timestamp, price))
                }));
            }
        }
        for series in out.values_mut() {
            series.sort_by_key(|(t, _)| *t);
            series.dedup_by_key(|(t, _)| *t);
        }
        Ok(out)
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
