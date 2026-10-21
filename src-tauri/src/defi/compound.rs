//! Compound v3 (Comet): base supply or borrow per market plus the collateral
//! flagged in `userBasic.assetsIn`, with rates read from the market itself.

use std::sync::Arc;

use super::u256::U256;
use async_trait::async_trait;
use rust_decimal::Decimal;

use super::{
    Position, PositionAdapter, PositionKind, PositionToken, SECONDS_PER_YEAR, TokenRole,
    aave::market_name,
    abi::{Arg, Call, Words, eth_calls, to_decimal},
    apr_to_apy,
};
use crate::{chains::Chain, error::Result};

fn markets(chain: Chain) -> &'static [&'static str] {
    match chain {
        Chain::Ethereum => &[
            "0xc3d688b66703497daa19211eedff47f25384cdc3",
            "0xa17581a9e3356d9a858b789d68b4d866e593ae94",
            "0x3afdc9bca9213a35503b077a6072f3d0d5ab0840",
            "0x3d0bb1ccab520a66e607822fc55bc921738fafe3",
        ],
        Chain::Arbitrum => &[
            "0x9c4ec768c28520b50860ea7a15bd7213a9ff58bf",
            "0xa5edbdd9646f8dff606d7448e414884c7d905dca",
            "0x6f7d514bbd4aff3bcd1140b7344b32f063dee486",
            "0xd98be00b5d27fc98112bde293e487f8d4ca57d07",
        ],
        Chain::Base => &[
            "0xb125e6687d4313864e53df431d5425969c15eb2f",
            "0x9c4ec768c28520b50860ea7a15bd7213a9ff58bf",
            "0x46e6b214b524310239732d51387075e0e70970bf",
        ],
        Chain::Optimism => &[
            "0x2e44e174f7d53f0212823acc11c01a11d58c5bcb",
            "0x995e394b8b2437ac8ce61ee0bc610d617962b214",
            "0xe36a30d249f7761327fd973001a32010b521b6fd",
        ],
        Chain::Polygon => &[
            "0xf25212e676d1f7f89cd72ffee66158f541246445",
            "0xaeb318360f27748acb200ce616e389a6c9409a07",
        ],
        _ => &[],
    }
}

pub fn adapters(http: &reqwest::Client) -> Vec<Arc<dyn PositionAdapter>> {
    [
        Chain::Ethereum,
        Chain::Arbitrum,
        Chain::Base,
        Chain::Optimism,
        Chain::Polygon,
    ]
    .into_iter()
    .map(|chain| {
        Arc::new(CompoundV3 {
            chain,
            http: http.clone(),
        }) as _
    })
    .collect()
}

pub struct CompoundV3 {
    chain: Chain,
    http: reqwest::Client,
}

/// Indexes of the collateral assets set in the `assetsIn` bitmap.
fn assets_in(bitmap: U256) -> Vec<u64> {
    (0..16u64).filter(|i| bitmap.bit(*i as usize)).collect()
}

/// Decimals of a collateral from its `scale` (10^decimals).
fn scale_decimals(scale: U256) -> Option<u32> {
    let digits = scale.to_string();
    (digits.starts_with('1') && digits[1..].bytes().all(|b| b == b'0'))
        .then(|| u32::try_from(digits.len() - 1).ok())
        .flatten()
}

/// Comet rates are per-second, scaled by 1e18.
fn per_second_to_apy(rate: U256) -> Option<Decimal> {
    let per_second = rate.to_f64() / 1e18;
    apr_to_apy(per_second * SECONDS_PER_YEAR)
}

struct Active<'a> {
    comet: &'a str,
    supplied: U256,
    borrowed: U256,
    collateral: Vec<u64>,
}

fn word(slot: &Option<Words>, i: usize) -> Option<U256> {
    slot.as_ref().and_then(|w| w.uint(i))
}

#[async_trait]
impl PositionAdapter for CompoundV3 {
    fn source(&self) -> String {
        format!("compound-v3:{}", self.chain)
    }

    fn chain(&self) -> Chain {
        self.chain
    }

    async fn positions(&self, owner: &str) -> Result<Vec<Position>> {
        let comets = markets(self.chain);
        let me = [Arg::Address(owner)];
        let basics = eth_calls(
            &self.http,
            self.chain,
            comets
                .iter()
                .flat_map(|c| {
                    [
                        "userBasic(address)",
                        "balanceOf(address)",
                        "borrowBalanceOf(address)",
                    ]
                    .map(|sig| Call::new(*c, sig, &me))
                })
                .collect(),
        )
        .await?;
        let active: Vec<Active<'_>> = comets
            .iter()
            .zip(basics.chunks(3))
            .filter_map(|(comet, r)| {
                let a = Active {
                    comet,
                    supplied: word(&r[1], 0).unwrap_or_default(),
                    borrowed: word(&r[2], 0).unwrap_or_default(),
                    collateral: word(&r[0], 3).map(assets_in).unwrap_or_default(),
                };
                (!a.supplied.is_zero() || !a.borrowed.is_zero() || !a.collateral.is_empty())
                    .then_some(a)
            })
            .collect();
        if active.is_empty() {
            return Ok(Vec::new());
        }

        // Market metadata and collateral asset descriptors.
        let mut calls = Vec::new();
        for a in &active {
            for sig in ["baseToken()", "decimals()", "getUtilization()"] {
                calls.push(Call::new(a.comet, sig, &[]));
            }
            for i in &a.collateral {
                calls.push(Call::new(a.comet, "getAssetInfo(uint8)", &[Arg::Uint(*i)]));
            }
        }
        let meta = eth_calls(&self.http, self.chain, calls).await?;

        struct Market<'a> {
            active: &'a Active<'a>,
            base: String,
            decimals: u32,
            utilization: U256,
            /// (asset, decimals, liquidation threshold)
            collateral: Vec<(String, u32, Option<Decimal>)>,
        }
        let mut markets = Vec::new();
        let mut cursor = meta.into_iter();
        for a in &active {
            let base = cursor.next().flatten().and_then(|w| w.address(0));
            let decimals = cursor.next().flatten().and_then(|w| w.uint(0));
            let utilization = cursor.next().flatten().and_then(|w| w.uint(0));
            let collateral = a
                .collateral
                .iter()
                .filter_map(|_| {
                    let info = cursor.next().flatten()?;
                    Some((
                        info.address(1)?,
                        scale_decimals(info.uint(3)?)?,
                        info.amount(5, 18),
                    ))
                })
                .collect();
            let (Some(base), Some(decimals)) = (base, decimals.and_then(|d| u32::try_from(d).ok()))
            else {
                continue;
            };
            markets.push(Market {
                active: a,
                base,
                decimals,
                utilization: utilization.unwrap_or_default(),
                collateral,
            });
        }

        // Rates, symbols and collateral balances.
        let mut calls = Vec::new();
        for m in &markets {
            let util = m.utilization.to_string().parse::<u64>().unwrap_or(0);
            calls.push(Call::new(
                m.active.comet,
                "getSupplyRate(uint256)",
                &[Arg::Uint(util)],
            ));
            calls.push(Call::new(
                m.active.comet,
                "getBorrowRate(uint256)",
                &[Arg::Uint(util)],
            ));
            calls.push(Call::new(m.base.as_str(), "symbol()", &[]));
            for (asset, ..) in &m.collateral {
                calls.push(Call::new(
                    m.active.comet,
                    "collateralBalanceOf(address,address)",
                    &[Arg::Address(owner), Arg::Address(asset)],
                ));
                calls.push(Call::new(asset.as_str(), "symbol()", &[]));
            }
        }
        let details = eth_calls(&self.http, self.chain, calls).await?;
        let mut cursor = details.into_iter();
        let symbol_of = |w: Option<Words>, contract: &str| {
            w.and_then(|w| w.string())
                .unwrap_or_else(|| contract.chars().take(6).collect())
        };

        let mut positions = Vec::new();
        for m in markets {
            let supply_apy = cursor
                .next()
                .flatten()
                .and_then(|w| w.uint(0))
                .and_then(per_second_to_apy);
            let borrow_apy = cursor
                .next()
                .flatten()
                .and_then(|w| w.uint(0))
                .and_then(per_second_to_apy);
            let base_symbol = symbol_of(cursor.next().flatten(), &m.base);
            let mut position = Position::new(
                self.chain,
                "compound-v3",
                PositionKind::Lending,
                String::new(),
            );
            for (asset, decimals, threshold) in &m.collateral {
                let amount = cursor
                    .next()
                    .flatten()
                    .and_then(|w| w.amount(0, *decimals))
                    .unwrap_or_default();
                let symbol = symbol_of(cursor.next().flatten(), asset);
                if amount.is_zero() {
                    continue;
                }
                let mut token =
                    PositionToken::new(self.chain, asset, symbol, TokenRole::Supplied, amount);
                token.liquidation_threshold = *threshold;
                token.apy = Some(Decimal::ZERO);
                position.tokens.push(token);
            }
            let supplied = to_decimal(m.active.supplied, m.decimals).unwrap_or_default();
            let borrowed = to_decimal(m.active.borrowed, m.decimals).unwrap_or_default();
            for (amount, role, apy) in [
                (supplied, TokenRole::Supplied, supply_apy),
                (borrowed, TokenRole::Borrowed, borrow_apy),
            ] {
                if !amount.is_zero() {
                    let mut token =
                        PositionToken::new(self.chain, &m.base, base_symbol.clone(), role, amount);
                    token.apy = apy;
                    position.tokens.push(token);
                }
            }
            if position.tokens.is_empty() {
                continue;
            }
            position.name = format!("{base_symbol} · {}", market_name(&position.tokens));
            position
                .receipts
                .push(format!("{}:{}", self.chain, m.active.comet));
            positions.push(position);
        }
        Ok(positions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_collateral_bitmap() {
        assert_eq!(assets_in(U256::ZERO), Vec::<u64>::new());
        assert_eq!(assets_in(U256::from(0b1001_0010u32)), vec![1, 4, 7]);
    }

    #[test]
    fn derives_decimals_from_scale() {
        assert_eq!(scale_decimals(U256::from(1_000_000u64)), Some(6));
        assert_eq!(scale_decimals(U256::from(10u64.pow(18))), Some(18));
        assert_eq!(scale_decimals(U256::from(1u64)), Some(0));
        assert_eq!(scale_decimals(U256::from(1234u64)), None);
    }

    #[test]
    fn converts_per_second_rates() {
        // 1e18 * 5% / seconds per year
        let rate = U256::from(1_585_489_599u64);
        let apy = per_second_to_apy(rate).unwrap();
        assert_eq!(apy.round_dp(4), Decimal::new(513, 4));
    }
}
