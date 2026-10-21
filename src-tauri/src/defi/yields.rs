//! DefiLlama yields (yields.llama.fi): keyless pool APYs, used for liquid
//! staking and Uniswap pools. The full dump is ~11 MB, so only the projects we
//! read are kept, in memory, for a few hours.

use std::{
    sync::{Arc, OnceLock},
    time::{Duration, Instant},
};

use rust_decimal::{Decimal, prelude::FromPrimitive};
use serde::Deserialize;
use tokio::sync::Mutex;

use crate::{chains::Chain, error::Result};

const URL: &str = "https://yields.llama.fi/pools";
const TTL: Duration = Duration::from_secs(6 * 3600);

/// Projects whose pools we look up.
const PROJECTS: [&str; 7] = [
    "lido",
    "rocket-pool",
    "coinbase-wrapped-staked-eth",
    "marinade-liquid-staking",
    "jito-liquid-staking",
    "benqi-staked-avax",
    "uniswap-v3",
];

#[derive(Debug, Deserialize)]
struct Response {
    data: Vec<RawPool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawPool {
    chain: String,
    project: String,
    apy: Option<f64>,
    tvl_usd: Option<f64>,
    pool_meta: Option<String>,
    underlying_tokens: Option<Vec<Option<String>>>,
}

#[derive(Clone, Debug)]
struct Pool {
    chain: String,
    project: String,
    apy: f64,
    tvl_usd: f64,
    pool_meta: Option<String>,
    tokens: Vec<String>,
}

#[derive(Debug, Default)]
pub struct Yields {
    pools: Vec<Pool>,
}

/// DefiLlama's chain names.
pub fn chain_name(chain: Chain) -> &'static str {
    match chain {
        Chain::Ethereum => "Ethereum",
        Chain::Arbitrum => "Arbitrum",
        Chain::Base => "Base",
        Chain::Optimism => "Optimism",
        Chain::Polygon => "Polygon",
        Chain::Bsc => "BSC",
        Chain::Avalanche => "Avalanche",
        Chain::Solana => "Solana",
        Chain::Bitcoin => "Bitcoin",
        Chain::Tron => "Tron",
        Chain::Xrp => "XRPL",
    }
}

fn ratio(apy: f64) -> Option<Decimal> {
    Decimal::from_f64(apy / 100.0).map(|d| d.round_dp(6).normalize())
}

impl Yields {
    fn parse(response: Response) -> Self {
        let mut pools: Vec<Pool> = response
            .data
            .into_iter()
            .filter(|p| PROJECTS.contains(&p.project.as_str()))
            .filter_map(|p| {
                Some(Pool {
                    apy: p.apy.filter(|a| a.is_finite())?,
                    tvl_usd: p.tvl_usd.unwrap_or(0.0),
                    tokens: p
                        .underlying_tokens
                        .unwrap_or_default()
                        .into_iter()
                        .flatten()
                        .map(|t| t.to_lowercase())
                        .collect(),
                    chain: p.chain,
                    project: p.project,
                    pool_meta: p.pool_meta,
                })
            })
            .collect();
        // Several pools can match; the deepest one is the most representative.
        pools.sort_by(|a, b| b.tvl_usd.total_cmp(&a.tvl_usd));
        Self { pools }
    }

    /// APY of a staking project on its home chain.
    pub fn staking(&self, project: &str, chain: &str) -> Option<Decimal> {
        self.pools
            .iter()
            .find(|p| p.project == project && p.chain == chain)
            .and_then(|p| ratio(p.apy))
    }

    /// APY of the Uniswap v3 pool of two tokens at a fee tier.
    pub fn uniswap(
        &self,
        chain: Chain,
        token0: &str,
        token1: &str,
        fee_label: &str,
    ) -> Option<Decimal> {
        let (t0, t1) = (token0.to_lowercase(), token1.to_lowercase());
        self.pools
            .iter()
            .find(|p| {
                p.project == "uniswap-v3"
                    && p.chain == chain_name(chain)
                    && p.pool_meta.as_deref() == Some(fee_label)
                    && p.tokens.len() == 2
                    && p.tokens.contains(&t0)
                    && p.tokens.contains(&t1)
            })
            .and_then(|p| ratio(p.apy))
    }
}

type Cache = Mutex<Option<(Instant, Arc<Yields>)>>;

fn cache() -> &'static Cache {
    static CACHE: OnceLock<Cache> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(None))
}

/// Pool APYs, from memory when fetched in the last few hours.
pub async fn load(http: &reqwest::Client) -> Result<Arc<Yields>> {
    let mut cached = cache().lock().await;
    if let Some((at, yields)) = cached.as_ref() {
        if at.elapsed() < TTL {
            return Ok(yields.clone());
        }
    }
    let response: Response = http
        .get(URL)
        .timeout(Duration::from_secs(60))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let yields = Arc::new(Yields::parse(response));
    *cached = Some((Instant::now(), yields.clone()));
    Ok(yields)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_staking_and_lp_yields() {
        let body = r#"{"status": "success", "data": [
          {"chain": "Ethereum", "project": "lido", "symbol": "STETH", "tvlUsd": 2.3e10, "apy": 2.225,
           "poolMeta": null, "underlyingTokens": ["0x0000000000000000000000000000000000000000"]},
          {"chain": "Ethereum", "project": "uniswap-v3", "symbol": "USDC-WETH", "tvlUsd": 1.2e8, "apy": 9.4,
           "poolMeta": "0.05%", "underlyingTokens": ["0xA0b86991c6218b36c1d19d4a2e9eb0ce3606eb48",
             "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2"]},
          {"chain": "Ethereum", "project": "uniswap-v3", "symbol": "USDC-WETH", "tvlUsd": 5e7, "apy": 12.0,
           "poolMeta": "0.3%", "underlyingTokens": ["0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48",
             "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2"]},
          {"chain": "Ethereum", "project": "curve-dex", "symbol": "3CRV", "tvlUsd": 1e9, "apy": 1.0},
          {"chain": "Solana", "project": "jito-liquid-staking", "symbol": "JITOSOL", "tvlUsd": 1e9, "apy": null}
        ]}"#;
        let yields = Yields::parse(serde_json::from_str(body).unwrap());
        assert_eq!(yields.pools.len(), 3);
        assert_eq!(
            yields.staking("lido", "Ethereum"),
            Some(Decimal::new(2225, 5))
        );
        assert_eq!(yields.staking("jito-liquid-staking", "Solana"), None);
        assert_eq!(
            yields.uniswap(
                Chain::Ethereum,
                "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2",
                "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48",
                "0.3%"
            ),
            Some(Decimal::new(12, 2))
        );
        assert_eq!(yields.uniswap(Chain::Base, "0xa", "0xb", "0.3%"), None);
    }
}
