//! DeFi and staking positions: lending, liquidity and staking read straight
//! from protocol contracts over the same keyless RPCs as the balances.
//! Liquid staking tokens are already balances, so they are derived from the
//! stored holdings instead (see [`store`]).

mod aave;
pub mod abi;
mod compound;
mod lst;
mod solana_stake;
pub mod store;
pub mod u256;
mod uniswap;
pub mod yields;

use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use futures::future::join_all;
use rust_decimal::{Decimal, prelude::FromPrimitive};
use serde::{Deserialize, Serialize};

use crate::{
    chains::{Chain, Family},
    db::repo::Wallet,
    error::Result,
};

/// Adapters queried at once during a refresh; public RPCs throttle bursts.
const CONCURRENCY: usize = 4;
pub const SECONDS_PER_YEAR: f64 = 31_536_000.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PositionKind {
    Lending,
    Liquidity,
    Staking,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TokenRole {
    Supplied,
    Borrowed,
    Pooled,
    Fees,
    Staked,
}

/// One token inside a position.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PositionToken {
    pub symbol: String,
    /// Contract or mint; `None` for a native coin.
    pub contract: Option<String>,
    /// DefiLlama coin id used to value it.
    pub price_key: String,
    pub role: TokenRole,
    pub amount: Decimal,
    pub value_usd: Option<Decimal>,
    /// Yearly yield (supplied) or cost (borrowed) as a ratio.
    pub apy: Option<Decimal>,
    /// Collateral weight at which the position gets liquidated (0.83 = 83%).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub liquidation_threshold: Option<Decimal>,
}

impl PositionToken {
    pub fn new(
        chain: Chain,
        contract: &str,
        symbol: String,
        role: TokenRole,
        amount: Decimal,
    ) -> Self {
        Self {
            symbol,
            price_key: format!("{}:{}", chain.llama_prefix(), contract),
            contract: Some(contract.to_string()),
            role,
            amount,
            value_usd: None,
            apy: None,
            liquidation_threshold: None,
        }
    }
}

/// Price range of a concentrated liquidity position, in token1 per token0.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LpRange {
    pub lower: Decimal,
    pub upper: Decimal,
    pub current: Decimal,
    pub in_range: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Position {
    #[serde(default)]
    pub wallet_id: i64,
    pub chain: Chain,
    /// Protocol id, e.g. `aave-v3`, `uniswap-v3`, `lido`.
    pub protocol: String,
    pub kind: PositionKind,
    /// Short description: tokens of the market or pool, NFT id…
    pub name: String,
    pub tokens: Vec<PositionToken>,
    /// Net value (assets minus debt); `None` until priced.
    pub value_usd: Option<Decimal>,
    /// Net yearly yield on `value_usd`, as a ratio.
    pub apy: Option<Decimal>,
    /// Lending health factor; liquidation happens below 1.
    pub health_factor: Option<Decimal>,
    /// The value is already part of the wallet balances (liquid staking
    /// tokens), so it must not be added to the portfolio total again.
    pub in_balances: bool,
    /// Asset ids in `balances` that only represent this position (aTokens,
    /// debt tokens, Comet shares); hidden from holdings to avoid counting twice.
    #[serde(default)]
    pub receipts: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub range: Option<LpRange>,
    /// Uniswap fee tier in hundredths of a bip (500 = 0.05%).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fee_tier: Option<u32>,
    /// Impermanent loss against holding the deposit. Needs the cost basis,
    /// which arrives with transaction history; `None` means "n/a".
    pub impermanent_loss: Option<Decimal>,
}

impl Position {
    pub fn new(chain: Chain, protocol: &str, kind: PositionKind, name: String) -> Self {
        Self {
            wallet_id: 0,
            chain,
            protocol: protocol.into(),
            kind,
            name,
            tokens: Vec::new(),
            value_usd: None,
            apy: None,
            health_factor: None,
            in_balances: false,
            receipts: Vec::new(),
            range: None,
            fee_tier: None,
            impermanent_loss: None,
        }
    }

    /// Fills token values from `prices` (by DefiLlama key), then the net
    /// value, net APY and — for lending without an on-chain figure — the
    /// health factor from the collateral thresholds.
    pub fn value_with(&mut self, prices: &HashMap<String, Decimal>) {
        let mut assets = Decimal::ZERO;
        let mut debt = Decimal::ZERO;
        let mut weighted_yield = Decimal::ZERO;
        let mut liquidation_capacity = Decimal::ZERO;
        let mut priced = false;
        for token in &mut self.tokens {
            let Some(price) = lookup(prices, &token.price_key) else {
                token.value_usd = None;
                continue;
            };
            let value = (token.amount * price).round_dp(8);
            token.value_usd = Some(value);
            priced = true;
            let apy = token.apy.unwrap_or_default();
            if token.role == TokenRole::Borrowed {
                debt += value;
                weighted_yield -= value * apy;
            } else {
                assets += value;
                weighted_yield += value * apy;
                if let Some(threshold) = token.liquidation_threshold {
                    liquidation_capacity += value * threshold;
                }
            }
        }
        if !priced {
            self.value_usd = None;
            return;
        }
        let net = assets - debt;
        self.value_usd = Some(net.round_dp(8).normalize());
        let has_rates = self.tokens.iter().any(|t| t.apy.is_some());
        if has_rates && !net.is_zero() {
            self.apy = Some((weighted_yield / net).round_dp(6).normalize());
        }
        if self.health_factor.is_none() && !debt.is_zero() && !liquidation_capacity.is_zero() {
            self.health_factor = Some((liquidation_capacity / debt).round_dp(4).normalize());
        }
    }
}

fn lookup(prices: &HashMap<String, Decimal>, key: &str) -> Option<Decimal> {
    prices.get(key).copied().or_else(|| {
        prices
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| *v)
    })
}

/// Continuous-compounding APY from a yearly rate.
pub fn apr_to_apy(apr: f64) -> Option<Decimal> {
    if !apr.is_finite() || apr < 0.0 {
        return None;
    }
    let apy = (SECONDS_PER_YEAR * (apr / SECONDS_PER_YEAR).ln_1p()).exp_m1();
    Decimal::from_f64(apy).map(|d| d.round_dp(6).normalize())
}

/// Reads the positions of one protocol on one chain.
#[async_trait]
pub trait PositionAdapter: Send + Sync {
    /// Identifies the adapter and chain; a successful fetch replaces every
    /// stored position of the wallet with this source.
    fn source(&self) -> String;

    fn chain(&self) -> Chain;

    async fn positions(&self, owner: &str) -> Result<Vec<Position>>;
}

/// Every adapter that applies to an address of `family`.
pub fn adapters(family: Family, http: &reqwest::Client) -> Vec<Arc<dyn PositionAdapter>> {
    match family {
        Family::Evm => aave::adapters(http)
            .into_iter()
            .chain(compound::adapters(http))
            .chain(uniswap::adapters(http))
            .collect(),
        Family::Solana => vec![Arc::new(solana_stake::SolanaStake::new(http.clone()))],
        Family::Bitcoin | Family::Tron | Family::Xrpl => Vec::new(),
    }
}

/// Positions fetched for one wallet from one source.
pub struct SourcePositions {
    pub wallet_id: i64,
    pub source: String,
    pub positions: Vec<Position>,
}

/// A source that failed; its stored positions keep their previous values.
pub struct SourceFailure {
    pub wallet_id: i64,
    pub chain: Chain,
    pub message: String,
}

#[derive(Default)]
pub struct DefiFetch {
    pub fetched: Vec<SourcePositions>,
    pub failures: Vec<SourceFailure>,
}

impl DefiFetch {
    /// DefiLlama keys of every token in the fetched positions.
    pub fn price_keys(&self) -> impl Iterator<Item = String> + '_ {
        self.fetched
            .iter()
            .flat_map(|s| &s.positions)
            .flat_map(|p| &p.tokens)
            .map(|t| t.price_key.clone())
    }
}

/// Queries every applicable adapter for `wallets`. Xpub wallets have no
/// contract positions.
pub async fn fetch(http: &reqwest::Client, wallets: &[Wallet]) -> DefiFetch {
    let jobs: Vec<(i64, String, Arc<dyn PositionAdapter>)> = wallets
        .iter()
        .filter(|w| w.kind == crate::db::repo::WalletKind::Address)
        .flat_map(|w| {
            adapters(w.family, http)
                .into_iter()
                .map(|a| (w.id, w.address.clone(), a))
        })
        .collect();
    let mut results = Vec::with_capacity(jobs.len());
    for group in jobs.chunks(CONCURRENCY) {
        let fetched = join_all(
            group
                .iter()
                .map(|(_, address, adapter)| adapter.positions(address)),
        )
        .await;
        results.extend(
            group
                .iter()
                .zip(fetched)
                .map(|((wallet_id, _, adapter), result)| (*wallet_id, adapter, result)),
        );
    }

    let mut out = DefiFetch::default();
    for (wallet_id, adapter, result) in results {
        match result {
            Ok(positions) => out.fetched.push(SourcePositions {
                wallet_id,
                source: adapter.source(),
                positions: positions
                    .into_iter()
                    .map(|mut p| {
                        p.wallet_id = wallet_id;
                        p
                    })
                    .collect(),
            }),
            Err(err) => out.failures.push(SourceFailure {
                wallet_id,
                chain: adapter.chain(),
                message: format!("{}: {err}", adapter.source()),
            }),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    fn token(symbol: &str, role: TokenRole, amount: &str, apy: Option<&str>) -> PositionToken {
        let mut t = PositionToken::new(
            Chain::Ethereum,
            &format!("0x{symbol}"),
            symbol.into(),
            role,
            Decimal::from_str(amount).unwrap(),
        );
        t.apy = apy.map(|a| Decimal::from_str(a).unwrap());
        t
    }

    #[test]
    fn values_lending_positions() {
        let mut p = Position::new(
            Chain::Ethereum,
            "compound-v3",
            PositionKind::Lending,
            "USDC".into(),
        );
        let mut weth = token("weth", TokenRole::Supplied, "2", None);
        weth.liquidation_threshold = Some(Decimal::from_str("0.85").unwrap());
        p.tokens = vec![
            weth,
            token("usdc", TokenRole::Supplied, "1000", Some("0.05")),
            token("usdc", TokenRole::Borrowed, "2000", Some("0.08")),
        ];
        let prices = HashMap::from([
            ("ethereum:0xweth".to_string(), Decimal::from(3000)),
            ("ethereum:0xUSDC".to_string(), Decimal::ONE),
        ]);
        p.value_with(&prices);
        // 6000 + 1000 - 2000
        assert_eq!(p.value_usd, Some(Decimal::from(5000)));
        // (1000 * 5% - 2000 * 8%) / 5000
        assert_eq!(p.apy, Some(Decimal::from_str("-0.022").unwrap()));
        // 6000 * 0.85 / 2000
        assert_eq!(p.health_factor, Some(Decimal::from_str("2.55").unwrap()));
        assert_eq!(p.tokens[2].value_usd, Some(Decimal::from(2000)));
    }

    #[test]
    fn unpriced_positions_have_no_value() {
        let mut p = Position::new(
            Chain::Base,
            "uniswap-v3",
            PositionKind::Liquidity,
            "X/Y".into(),
        );
        p.tokens = vec![token("x", TokenRole::Pooled, "1", None)];
        p.value_with(&HashMap::new());
        assert_eq!(p.value_usd, None);
        assert_eq!(p.apy, None);
    }

    #[test]
    fn converts_rates_to_apy() {
        assert_eq!(apr_to_apy(0.0), Some(Decimal::ZERO));
        // e^0.05 - 1
        assert_eq!(
            apr_to_apy(0.05),
            Some(Decimal::from_str("0.051271").unwrap())
        );
        assert_eq!(apr_to_apy(f64::NAN), None);
    }
}
