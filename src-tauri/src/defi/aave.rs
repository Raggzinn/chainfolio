//! Aave v3: account totals and health factor from `Pool.getUserAccountData`,
//! then per-reserve aToken and variable debt balances with on-chain rates.

use std::sync::Arc;

use super::u256::U256;
use async_trait::async_trait;
use rust_decimal::Decimal;

use super::{
    Position, PositionAdapter, PositionKind, PositionToken, TokenRole,
    abi::{Arg, Call, Words, eth_call, eth_calls, to_decimal},
    apr_to_apy,
};
use crate::{chains::Chain, error::Result};

const POOLS: [(Chain, &str); 6] = [
    (
        Chain::Ethereum,
        "0x87870bca3f3fd6335c3f4ce8392d69350b4fa4e2",
    ),
    (
        Chain::Arbitrum,
        "0x794a61358d6845594f94dc1db02a252b5b4814ad",
    ),
    (Chain::Base, "0xa238dd80c259a72e81d7e4664a9801593f98d1c5"),
    (
        Chain::Optimism,
        "0x794a61358d6845594f94dc1db02a252b5b4814ad",
    ),
    (Chain::Polygon, "0x794a61358d6845594f94dc1db02a252b5b4814ad"),
    (
        Chain::Avalanche,
        "0x794a61358d6845594f94dc1db02a252b5b4814ad",
    ),
];

pub fn adapters(http: &reqwest::Client) -> Vec<Arc<dyn PositionAdapter>> {
    POOLS
        .iter()
        .map(|&(chain, pool)| {
            Arc::new(AaveV3 {
                chain,
                pool,
                http: http.clone(),
            }) as _
        })
        .collect()
}

pub struct AaveV3 {
    chain: Chain,
    pool: &'static str,
    http: reqwest::Client,
}

/// `getReserveData` fields we use (the legacy 15-word struct).
#[derive(Debug, PartialEq)]
struct Reserve {
    decimals: u32,
    liquidity_rate: U256,
    variable_borrow_rate: U256,
    a_token: String,
    variable_debt_token: String,
}

fn parse_reserve(words: &Words) -> Option<Reserve> {
    if words.len() < 11 {
        return None;
    }
    let config = words.uint(0)?;
    // ReserveConfigurationMap: bits 48–55 hold the decimals.
    let decimals = u32::try_from((config >> 48) & U256::from(0xffu8)).ok()?;
    Some(Reserve {
        decimals,
        liquidity_rate: words.uint(2)?,
        variable_borrow_rate: words.uint(4)?,
        a_token: words.address(8)?,
        variable_debt_token: words.address(10)?,
    })
}

/// Health factor from `getUserAccountData` (18 decimals). Accounts without
/// debt report `type(uint256).max`, which means "cannot be liquidated".
pub fn parse_health_factor(raw: U256) -> Option<Decimal> {
    if raw >= U256::from(1u128 << 100) {
        return None;
    }
    to_decimal(raw, 18).map(|hf| hf.round_dp(4).normalize())
}

/// Aave rates are yearly APRs in ray (27 decimals), compounded per second.
fn ray_to_apy(rate: U256) -> Option<Decimal> {
    let apr = rate.to_f64() / 1e27;
    apr_to_apy(apr)
}

#[async_trait]
impl PositionAdapter for AaveV3 {
    fn source(&self) -> String {
        format!("aave-v3:{}", self.chain)
    }

    fn chain(&self) -> Chain {
        self.chain
    }

    async fn positions(&self, owner: &str) -> Result<Vec<Position>> {
        let account = eth_call(
            &self.http,
            self.chain,
            Call::new(
                self.pool,
                "getUserAccountData(address)",
                &[Arg::Address(owner)],
            ),
        )
        .await?;
        let (Some(collateral), Some(debt)) = (account.uint(0), account.uint(1)) else {
            return Ok(Vec::new());
        };
        // Supplies not enabled as collateral are missed by this shortcut, but
        // it saves ~200 calls per chain for the vast majority of addresses.
        if collateral.is_zero() && debt.is_zero() {
            return Ok(Vec::new());
        }

        let reserves = eth_call(
            &self.http,
            self.chain,
            Call::new(self.pool, "getReservesList()", &[]),
        )
        .await?
        .addresses()
        .unwrap_or_default();
        let data = eth_calls(
            &self.http,
            self.chain,
            reserves
                .iter()
                .map(|r| Call::new(self.pool, "getReserveData(address)", &[Arg::Address(r)]))
                .collect(),
        )
        .await?;
        let reserves: Vec<(String, Reserve)> = reserves
            .into_iter()
            .zip(data)
            .filter_map(|(asset, words)| Some((asset, parse_reserve(&words?)?)))
            .collect();

        let balance_calls = reserves
            .iter()
            .flat_map(|(_, r)| {
                [&r.a_token, &r.variable_debt_token].map(|token| {
                    Call::new(token.as_str(), "balanceOf(address)", &[Arg::Address(owner)])
                })
            })
            .collect();
        let balances = eth_calls(&self.http, self.chain, balance_calls).await?;
        let held: Vec<(&String, &Reserve, Decimal, Decimal)> = reserves
            .iter()
            .zip(balances.chunks(2))
            .filter_map(|((asset, reserve), pair)| {
                let amount = |slot: &Option<Words>| {
                    slot.as_ref()
                        .and_then(|w| w.amount(0, reserve.decimals))
                        .unwrap_or_default()
                };
                let (supplied, borrowed) = (amount(&pair[0]), amount(&pair[1]));
                (!supplied.is_zero() || !borrowed.is_zero())
                    .then_some((asset, reserve, supplied, borrowed))
            })
            .collect();
        if held.is_empty() {
            return Ok(Vec::new());
        }

        let symbols = eth_calls(
            &self.http,
            self.chain,
            held.iter()
                .map(|(asset, ..)| Call::new(asset.as_str(), "symbol()", &[]))
                .collect(),
        )
        .await?;

        let mut position =
            Position::new(self.chain, "aave-v3", PositionKind::Lending, String::new());
        position.health_factor = account.uint(5).and_then(parse_health_factor);
        for ((asset, reserve, supplied, borrowed), symbol) in held.into_iter().zip(symbols) {
            let symbol = symbol
                .and_then(|w| w.string())
                .unwrap_or_else(|| asset.chars().take(6).collect());
            if !supplied.is_zero() {
                let mut token = PositionToken::new(
                    self.chain,
                    asset,
                    symbol.clone(),
                    TokenRole::Supplied,
                    supplied,
                );
                token.apy = ray_to_apy(reserve.liquidity_rate);
                position.tokens.push(token);
                position
                    .receipts
                    .push(format!("{}:{}", self.chain, reserve.a_token));
            }
            if !borrowed.is_zero() {
                let mut token =
                    PositionToken::new(self.chain, asset, symbol, TokenRole::Borrowed, borrowed);
                token.apy = ray_to_apy(reserve.variable_borrow_rate);
                position.tokens.push(token);
                position
                    .receipts
                    .push(format!("{}:{}", self.chain, reserve.variable_debt_token));
            }
        }
        position.name = market_name(&position.tokens);
        Ok(vec![position])
    }
}

/// "WETH, wstETH → USDC": supplied symbols, then borrowed ones.
pub fn market_name(tokens: &[PositionToken]) -> String {
    let list = |borrowed: bool| {
        let mut symbols: Vec<&str> = tokens
            .iter()
            .filter(|t| (t.role == TokenRole::Borrowed) == borrowed)
            .map(|t| t.symbol.as_str())
            .collect();
        symbols.dedup();
        symbols.join(", ")
    };
    let (supplied, borrowed) = (list(false), list(true));
    if borrowed.is_empty() {
        supplied
    } else {
        format!("{supplied} → {borrowed}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn word_hex(n: U256) -> String {
        hex::encode(n.to_be_bytes())
    }

    #[test]
    fn parses_health_factors() {
        let max = U256::MAX;
        assert_eq!(parse_health_factor(max), None);
        let hf = U256::from(1_523_456_789_000_000_000u128);
        assert_eq!(parse_health_factor(hf), Some(Decimal::new(15235, 4)));
        let liquidatable = U256::from(980_000_000_000_000_000u128);
        assert_eq!(parse_health_factor(liquidatable), Some(Decimal::new(98, 2)));
    }

    #[test]
    fn parses_reserve_data() {
        // decimals = 6 at bits 48..56, plus unrelated LTV bits.
        let config = (U256::from(6u8) << 48) | U256::from(0x1f40u64);
        let ray = 10u128.pow(27);
        let mut hex = String::from("0x");
        hex += &word_hex(config);
        hex += &word_hex(U256::from(1u8)); // liquidity index
        hex += &word_hex(U256::from(ray / 20)); // 5% supply APR
        hex += &word_hex(U256::from(1u8)); // borrow index
        hex += &word_hex(U256::from(ray / 10)); // 10% borrow APR
        for i in 5..15 {
            let word = match i {
                8 => format!("{:0>64}", "aaaa"),
                10 => format!("{:0>64}", "dddd"),
                _ => word_hex(U256::ZERO),
            };
            hex += &word;
        }
        let reserve = parse_reserve(&Words::parse(&hex).unwrap()).unwrap();
        assert_eq!(reserve.decimals, 6);
        assert!(reserve.a_token.ends_with("aaaa"));
        assert!(reserve.variable_debt_token.ends_with("dddd"));
        assert_eq!(ray_to_apy(reserve.liquidity_rate), apr_to_apy(0.05));
        assert_eq!(
            ray_to_apy(reserve.variable_borrow_rate)
                .unwrap()
                .round_dp(4),
            Decimal::new(1052, 4)
        );
    }

    #[test]
    fn names_markets() {
        let t =
            |s: &str, role| PositionToken::new(Chain::Base, "0x1", s.into(), role, Decimal::ONE);
        assert_eq!(
            market_name(&[
                t("WETH", TokenRole::Supplied),
                t("cbETH", TokenRole::Supplied),
                t("USDC", TokenRole::Borrowed)
            ]),
            "WETH, cbETH → USDC"
        );
        assert_eq!(market_name(&[t("USDC", TokenRole::Supplied)]), "USDC");
    }
}
