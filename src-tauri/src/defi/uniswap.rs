//! Uniswap v3 liquidity NFTs: positions from the NonfungiblePositionManager,
//! token amounts from the pool price and uncollected fees from fee growth.

use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

use super::u256::U256;
use async_trait::async_trait;
use rust_decimal::{Decimal, prelude::FromPrimitive};

use super::{
    LpRange, Position, PositionAdapter, PositionKind, PositionToken, TokenRole,
    abi::{Arg, Call, Words, eth_call, eth_calls, to_decimal},
};
use crate::{chains::Chain, error::Result};

/// Positions read per chain; LPs that rebalance a lot own thousands of
/// closed NFTs, and the newest ones are the likeliest to be open.
const MAX_NFTS: u64 = 60;

/// (NonfungiblePositionManager, factory)
fn contracts(chain: Chain) -> Option<(&'static str, &'static str)> {
    const NPM: &str = "0xc36442b4a4522e871399cd717abdd847ab11fe88";
    const FACTORY: &str = "0x1f98431c8ad98523631ae4a59f267346ea31f984";
    match chain {
        Chain::Ethereum | Chain::Arbitrum | Chain::Optimism | Chain::Polygon => {
            Some((NPM, FACTORY))
        }
        Chain::Base => Some((
            "0x03a520b32c04bf3beef7beb72e919cf822ed34f1",
            "0x33128a8fc17869897dce68ed026d694621f6fdfd",
        )),
        Chain::Avalanche => Some((
            "0x655c406ebfa14ee2006250925e54ec43ad184f8b",
            "0x740b1c1de25031c31ff4fc9a62f554a55cdc1bad",
        )),
        Chain::Bsc => Some((
            "0x7b8a01b39d58278b5de7e48c8449c9f4f5170613",
            "0xdb1d10011ad0ff90774d0c6bb92e5c5c8b4461f7",
        )),
        _ => None,
    }
}

pub fn adapters(http: &reqwest::Client) -> Vec<Arc<dyn PositionAdapter>> {
    Chain::EVM
        .into_iter()
        .filter_map(|chain| {
            let (npm, factory) = contracts(chain)?;
            Some(Arc::new(UniswapV3 {
                chain,
                npm,
                factory,
                http: http.clone(),
            }) as _)
        })
        .collect()
}

pub struct UniswapV3 {
    chain: Chain,
    npm: &'static str,
    factory: &'static str,
    http: reqwest::Client,
}

/// `NonfungiblePositionManager.positions(tokenId)`.
#[derive(Clone, Debug, PartialEq)]
pub struct NftPosition {
    pub id: u64,
    pub token0: String,
    pub token1: String,
    pub fee: u32,
    pub tick_lower: i64,
    pub tick_upper: i64,
    pub liquidity: u128,
    pub fee_inside_last: [U256; 2],
    pub owed: [U256; 2],
}

fn parse_position(id: u64, w: &Words) -> Option<NftPosition> {
    Some(NftPosition {
        id,
        token0: w.address(2)?,
        token1: w.address(3)?,
        fee: u32::try_from(w.uint(4)?).ok()?,
        tick_lower: w.int(5)?,
        tick_upper: w.int(6)?,
        liquidity: u128::try_from(w.uint(7)?).ok()?,
        fee_inside_last: [w.uint(8)?, w.uint(9)?],
        owed: [w.uint(10)?, w.uint(11)?],
    })
}

/// What the pool reports: price, current tick and global fee growth.
#[derive(Clone, Debug, PartialEq)]
pub struct PoolState {
    pub sqrt_price_x96: U256,
    pub tick: i64,
    pub fee_growth_global: [U256; 2],
}

fn sqrt_price_at_tick(tick: i64) -> f64 {
    1.0001f64.powf(tick as f64 / 2.0)
}

/// Token amounts (in base units) of `liquidity` between two ticks at the
/// current square-root price, per the Uniswap v3 whitepaper (eq. 6.29/6.30).
pub fn amounts(liquidity: f64, sqrt_price: f64, tick_lower: i64, tick_upper: i64) -> (f64, f64) {
    let (sa, sb) = (
        sqrt_price_at_tick(tick_lower),
        sqrt_price_at_tick(tick_upper),
    );
    if sqrt_price <= sa {
        (liquidity * (sb - sa) / (sa * sb), 0.0)
    } else if sqrt_price >= sb {
        (0.0, liquidity * (sb - sa))
    } else {
        (
            liquidity * (sb - sqrt_price) / (sqrt_price * sb),
            liquidity * (sqrt_price - sa),
        )
    }
}

/// Uncollected fees in base units of each token: what the NFT already owes
/// plus fees earned inside the range since its last update. Fee growth
/// counters wrap around by design, so all arithmetic is modular.
pub fn uncollected_fees(
    position: &NftPosition,
    pool: &PoolState,
    lower_outside: [U256; 2],
    upper_outside: [U256; 2],
) -> [U256; 2] {
    std::array::from_fn(|i| {
        let global = pool.fee_growth_global[i];
        let below = if pool.tick >= position.tick_lower {
            lower_outside[i]
        } else {
            global.wrapping_sub(lower_outside[i])
        };
        let above = if pool.tick < position.tick_upper {
            upper_outside[i]
        } else {
            global.wrapping_sub(upper_outside[i])
        };
        let inside = global.wrapping_sub(below).wrapping_sub(above);
        let earned = inside
            .wrapping_sub(position.fee_inside_last[i])
            .mul_shr_128(position.liquidity);
        position.owed[i].wrapping_add(earned)
    })
}

/// Price of token0 in token1 at `tick`, adjusted for decimals.
fn price_at_tick(tick: i64, decimals: (u32, u32)) -> f64 {
    1.0001f64.powf(tick as f64) * 10f64.powi(decimals.0 as i32 - decimals.1 as i32)
}

fn dec(value: f64) -> Decimal {
    Decimal::from_f64(value)
        .map(|d| d.round_dp(10).normalize())
        .unwrap_or_default()
}

/// "0.05%" for a fee of 500.
pub fn fee_label(fee: u32) -> String {
    let pct = Decimal::new(i64::from(fee), 4).normalize();
    format!("{pct}%")
}

#[async_trait]
impl PositionAdapter for UniswapV3 {
    fn source(&self) -> String {
        format!("uniswap-v3:{}", self.chain)
    }

    fn chain(&self) -> Chain {
        self.chain
    }

    async fn positions(&self, owner: &str) -> Result<Vec<Position>> {
        let (http, chain) = (&self.http, self.chain);
        let count = eth_call(
            http,
            chain,
            Call::new(self.npm, "balanceOf(address)", &[Arg::Address(owner)]),
        )
        .await?
        .uint(0)
        .map_or(0, |n| u64::try_from(n).unwrap_or(u64::MAX));
        if count == 0 {
            return Ok(Vec::new());
        }
        let first = count.saturating_sub(MAX_NFTS);
        let ids: Vec<u64> = eth_calls(
            http,
            chain,
            (first..count)
                .map(|i| {
                    Call::new(
                        self.npm,
                        "tokenOfOwnerByIndex(address,uint256)",
                        &[Arg::Address(owner), Arg::Uint(i)],
                    )
                })
                .collect(),
        )
        .await?
        .into_iter()
        .filter_map(|w| u64::try_from(w?.uint(0)?).ok())
        .collect();
        let details = eth_calls(
            http,
            chain,
            ids.iter()
                .map(|id| Call::new(self.npm, "positions(uint256)", &[Arg::Uint(*id)]))
                .collect(),
        )
        .await?;
        let open: Vec<NftPosition> = ids
            .iter()
            .zip(details)
            .filter_map(|(id, w)| parse_position(*id, &w?))
            .filter(|p| p.liquidity != 0 || p.owed.iter().any(|o| !o.is_zero()))
            .collect();
        if open.is_empty() {
            return Ok(Vec::new());
        }

        // Pools and token metadata.
        let pool_keys: BTreeSet<(String, String, u32)> = open
            .iter()
            .map(|p| (p.token0.clone(), p.token1.clone(), p.fee))
            .collect();
        let tokens: BTreeSet<String> = open
            .iter()
            .flat_map(|p| [p.token0.clone(), p.token1.clone()])
            .collect();
        let mut calls: Vec<Call> = pool_keys
            .iter()
            .map(|(t0, t1, fee)| {
                Call::new(
                    self.factory,
                    "getPool(address,address,uint24)",
                    &[
                        Arg::Address(t0),
                        Arg::Address(t1),
                        Arg::Uint(u64::from(*fee)),
                    ],
                )
            })
            .collect();
        for token in &tokens {
            calls.push(Call::new(token.as_str(), "decimals()", &[]));
            calls.push(Call::new(token.as_str(), "symbol()", &[]));
        }
        let mut meta = eth_calls(http, chain, calls).await?.into_iter();
        let pools: BTreeMap<(String, String, u32), String> = pool_keys
            .into_iter()
            .filter_map(|key| Some((key, meta.next().flatten()?.address(0)?)))
            .collect();
        let token_meta: BTreeMap<String, (u32, String)> = tokens
            .into_iter()
            .filter_map(|token| {
                let decimals = meta.next().flatten().and_then(|w| w.uint(0));
                let symbol = meta.next().flatten().and_then(|w| w.string());
                let decimals = u32::try_from(decimals?).ok()?;
                let symbol = symbol.unwrap_or_else(|| token.chars().take(6).collect());
                Some((token, (decimals, symbol)))
            })
            .collect();

        // Pool state plus the tick boundaries of every position.
        let pool_list: Vec<&String> = pools
            .values()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let mut calls = Vec::new();
        for pool in &pool_list {
            for sig in [
                "slot0()",
                "feeGrowthGlobal0X128()",
                "feeGrowthGlobal1X128()",
            ] {
                calls.push(Call::new(pool.as_str(), sig, &[]));
            }
        }
        let mut position_pools = Vec::new();
        for p in &open {
            let pool = pools.get(&(p.token0.clone(), p.token1.clone(), p.fee));
            if let Some(pool) = pool {
                for tick in [p.tick_lower, p.tick_upper] {
                    calls.push(Call::new(pool.as_str(), "ticks(int24)", &[Arg::Int(tick)]));
                }
            }
            position_pools.push(pool);
        }
        let mut state = eth_calls(http, chain, calls).await?.into_iter();
        let mut pool_states: BTreeMap<&String, PoolState> = BTreeMap::new();
        for pool in pool_list {
            let slot0 = state.next().flatten();
            let g0 = state.next().flatten().and_then(|w| w.uint(0));
            let g1 = state.next().flatten().and_then(|w| w.uint(0));
            if let (Some(slot0), Some(g0), Some(g1)) = (slot0, g0, g1) {
                if let (Some(sqrt_price_x96), Some(tick)) = (slot0.uint(0), slot0.int(1)) {
                    pool_states.insert(
                        pool,
                        PoolState {
                            sqrt_price_x96,
                            tick,
                            fee_growth_global: [g0, g1],
                        },
                    );
                }
            }
        }

        let mut positions = Vec::new();
        for (p, pool) in open.iter().zip(position_pools) {
            let Some(pool) = pool else { continue };
            let lower = state.next().flatten();
            let upper = state.next().flatten();
            let (Some(pool_state), Some((dec0, sym0)), Some((dec1, sym1))) = (
                pool_states.get(pool),
                token_meta.get(&p.token0),
                token_meta.get(&p.token1),
            ) else {
                continue;
            };
            let outside = |w: &Option<Words>| -> [U256; 2] {
                let w = w.as_ref();
                [2, 3].map(|i| w.and_then(|w| w.uint(i)).unwrap_or_default())
            };
            let fees = uncollected_fees(p, pool_state, outside(&lower), outside(&upper));
            let sqrt_price = pool_state.sqrt_price_x96.to_f64() / 2f64.powi(96);
            let (raw0, raw1) = amounts(p.liquidity as f64, sqrt_price, p.tick_lower, p.tick_upper);

            let mut position = Position::new(
                chain,
                "uniswap-v3",
                PositionKind::Liquidity,
                format!("{sym0}/{sym1} {} #{}", fee_label(p.fee), p.id),
            );
            position.fee_tier = Some(p.fee);
            let scale = |raw: f64, decimals: u32| dec(raw / 10f64.powi(decimals as i32));
            for (token, symbol, decimals, raw, fee) in [
                (&p.token0, sym0, *dec0, raw0, fees[0]),
                (&p.token1, sym1, *dec1, raw1, fees[1]),
            ] {
                position.tokens.push(PositionToken::new(
                    chain,
                    token,
                    symbol.clone(),
                    TokenRole::Pooled,
                    scale(raw, decimals),
                ));
                let earned = to_decimal(fee, decimals).unwrap_or_default();
                if !earned.is_zero() {
                    position.tokens.push(PositionToken::new(
                        chain,
                        token,
                        symbol.clone(),
                        TokenRole::Fees,
                        earned,
                    ));
                }
            }
            let decimals = (*dec0, *dec1);
            position.range = Some(LpRange {
                lower: dec(price_at_tick(p.tick_lower, decimals)),
                upper: dec(price_at_tick(p.tick_upper, decimals)),
                current: dec(sqrt_price * sqrt_price * 10f64.powi(*dec0 as i32 - *dec1 as i32)),
                in_range: (p.tick_lower..p.tick_upper).contains(&pool_state.tick),
            });
            positions.push(position);
        }
        Ok(positions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, tolerance: f64) -> bool {
        ((a - b) / b).abs() < tolerance
    }

    #[test]
    fn computes_amounts_in_range() {
        // ETH/USDC example from the Uniswap v3 development book: 1 ETH and
        // 5000 USDC deposited between 4545 and 5500 at a price of 5000.
        let liquidity = 1_517_882_343_751_509_868_544f64;
        let sqrt_price = 5_602_277_097_478_614_198_912_276_234_240f64 / 2f64.powi(96);
        let (a0, a1) = amounts(liquidity, sqrt_price, 84_222, 86_129);
        assert!(close(a0, 0.998_976_618_347_425_3e18, 5e-3), "{a0}");
        assert!(close(a1, 5000e18, 5e-3), "{a1}");
    }

    #[test]
    fn out_of_range_positions_hold_one_token() {
        let l = 1e18;
        let below = sqrt_price_at_tick(-1000);
        let above = sqrt_price_at_tick(1000);
        let (a0, a1) = amounts(l, below, -100, 100);
        assert!(a0 > 0.0 && a1 == 0.0);
        let (sa, sb) = (sqrt_price_at_tick(-100), sqrt_price_at_tick(100));
        assert!(close(a0, l * (1.0 / sa - 1.0 / sb), 1e-12));
        let (a0, a1) = amounts(l, above, -100, 100);
        assert!(a0 == 0.0 && close(a1, l * (sb - sa), 1e-12));
    }

    fn q128(n: u64) -> U256 {
        U256::from(n) << 128
    }

    fn nft(liquidity: u64, last: [U256; 2], owed: [U256; 2]) -> NftPosition {
        NftPosition {
            id: 1,
            token0: "0xa".into(),
            token1: "0xb".into(),
            fee: 500,
            tick_lower: -100,
            tick_upper: 100,
            liquidity: u128::from(liquidity),
            fee_inside_last: last,
            owed,
        }
    }

    #[test]
    fn computes_uncollected_fees() {
        let pool = PoolState {
            sqrt_price_x96: U256::ZERO,
            tick: 0,
            fee_growth_global: [q128(1000), q128(10)],
        };
        let position = nft(2, [q128(300), U256::ZERO], [U256::from(7u64), U256::ZERO]);
        let fees = uncollected_fees(&position, &pool, [q128(100), q128(1)], [q128(200), q128(2)]);
        // inside0 = 1000 - 100 - 200 = 700; (700 - 300) * 2 + 7 owed
        assert_eq!(fees[0], U256::from(807u64));
        // inside1 = 10 - 1 - 2 = 7; 7 * 2
        assert_eq!(fees[1], U256::from(14u64));
    }

    #[test]
    fn fee_growth_wraps_around() {
        // Below the range: the lower tick's outside growth counts as above.
        let pool = PoolState {
            sqrt_price_x96: U256::ZERO,
            tick: -500,
            fee_growth_global: [q128(5), q128(5)],
        };
        let lower = [q128(10), q128(10)];
        let upper = [U256::ZERO, U256::ZERO];
        let last = U256::ZERO.wrapping_sub(q128(3));
        let fees = uncollected_fees(&nft(4, [last, last], [U256::ZERO; 2]), &pool, lower, upper);
        // below = 5 - 10 (wraps), inside = 5 - below = 10, minus last (-3) = 13 per unit
        assert_eq!(fees[0], U256::from(52u64));
    }

    #[test]
    fn labels_fee_tiers() {
        assert_eq!(fee_label(500), "0.05%");
        assert_eq!(fee_label(3000), "0.3%");
        assert_eq!(fee_label(100), "0.01%");
        assert_eq!(fee_label(10000), "1%");
    }
}
