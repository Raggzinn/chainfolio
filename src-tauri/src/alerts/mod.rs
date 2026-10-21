//! Alert rules, evaluated after every sync against the fresh prices and
//! balances. Evaluation is pure; `store` persists rules and history and
//! `commands` exposes them to the frontend.

pub mod commands;
pub mod store;

use std::collections::{HashMap, HashSet};

use rusqlite::Connection;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::{
    db::repo::{self, Portfolio, SyncWrite},
    error::Result,
    prices::{PriceProvider, defillama::DefiLlama},
    state::AppState,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Direction {
    Up,
    Down,
    Either,
}

/// What a rule watches. Ratios are fractions (0.05 = 5%), money is USD.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Condition {
    PriceAbove {
        price_key: String,
        symbol: String,
        price: Decimal,
    },
    PriceBelow {
        price_key: String,
        symbol: String,
        price: Decimal,
    },
    /// The portfolio moved at least `ratio` over the last 24h.
    PortfolioChange {
        direction: Direction,
        ratio: Decimal,
    },
    /// At least `usd` of value entered or left a wallet (any wallet when
    /// `wallet_id` is `None`) between two syncs.
    BalanceChange {
        wallet_id: Option<i64>,
        direction: Direction,
        usd: Decimal,
    },
    /// A held stablecoin trades at least `ratio` away from $1.
    Depeg { ratio: Decimal },
}

impl Condition {
    /// Rejects thresholds that could never (or would always) fire.
    pub fn validate(&self) -> bool {
        match self {
            Self::PriceAbove {
                price, price_key, ..
            }
            | Self::PriceBelow {
                price, price_key, ..
            } => *price > Decimal::ZERO && !price_key.is_empty(),
            Self::PortfolioChange { ratio, .. } | Self::Depeg { ratio } => {
                *ratio > Decimal::ZERO && *ratio < Decimal::ONE
            }
            Self::BalanceChange { usd, .. } => *usd > Decimal::ZERO,
        }
    }

    /// Price alerts may watch assets nobody holds, which the regular sync
    /// doesn't price.
    pub fn price_key(&self) -> Option<&str> {
        match self {
            Self::PriceAbove { price_key, .. } | Self::PriceBelow { price_key, .. } => {
                Some(price_key)
            }
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Rule {
    pub id: i64,
    pub condition: Condition,
    pub enabled: bool,
    /// False after a level condition fired and still holds.
    pub armed: bool,
    pub created_at: i64,
    pub last_triggered_at: Option<i64>,
}

/// USD that entered and left one wallet between two syncs (both positive).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Flow {
    pub incoming: Decimal,
    pub outgoing: Decimal,
}

/// What the latest sync learned, as seen by the rules.
#[derive(Debug, Default)]
pub struct Context {
    /// USD price per DefiLlama key.
    pub prices: HashMap<String, Decimal>,
    pub portfolio_change: Option<Decimal>,
    pub flows: HashMap<i64, Flow>,
    /// Held stablecoins: (symbol, USD price).
    pub stablecoins: Vec<(String, Decimal)>,
}

/// One firing of a rule.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Trigger {
    pub rule_id: i64,
    pub condition: Condition,
    /// The price, the 24h change or the USD that moved (negative = out).
    pub value: Decimal,
    /// Depegged symbol or wallet id the value refers to.
    pub subject: Option<String>,
}

/// Result of evaluating one rule.
#[derive(Debug, PartialEq)]
pub struct Outcome {
    pub rule_id: i64,
    pub armed: bool,
    pub triggers: Vec<Trigger>,
}

/// Whether a level condition holds right now.
enum Level {
    Holds {
        value: Decimal,
        subject: Option<String>,
    },
    Clear,
    /// Not enough data (e.g. no price yet): keep the previous state.
    Unknown,
}

/// Evaluates enabled rules. Level conditions fire on the sync where they
/// start to hold and re-arm once they stop holding; balance changes fire on
/// every sync that moves enough value.
pub fn evaluate(rules: &[Rule], ctx: &Context) -> Vec<Outcome> {
    rules
        .iter()
        .filter(|rule| rule.enabled)
        .map(|rule| {
            if let Condition::BalanceChange {
                wallet_id,
                direction,
                usd,
            } = &rule.condition
            {
                return Outcome {
                    rule_id: rule.id,
                    armed: true,
                    triggers: balance_triggers(rule, *wallet_id, *direction, *usd, ctx),
                };
            }
            let fire = |value, subject| Trigger {
                rule_id: rule.id,
                condition: rule.condition.clone(),
                value,
                subject,
            };
            match level(&rule.condition, ctx) {
                Level::Holds { value, subject } => Outcome {
                    rule_id: rule.id,
                    armed: false,
                    triggers: if rule.armed {
                        vec![fire(value, subject)]
                    } else {
                        Vec::new()
                    },
                },
                Level::Clear => Outcome {
                    rule_id: rule.id,
                    armed: true,
                    triggers: Vec::new(),
                },
                Level::Unknown => Outcome {
                    rule_id: rule.id,
                    armed: rule.armed,
                    triggers: Vec::new(),
                },
            }
        })
        .collect()
}

fn level(condition: &Condition, ctx: &Context) -> Level {
    let holds = |ok: bool, value: Decimal, subject: Option<String>| {
        if ok {
            Level::Holds { value, subject }
        } else {
            Level::Clear
        }
    };
    match condition {
        Condition::PriceAbove {
            price_key, price, ..
        } => match ctx.prices.get(price_key) {
            Some(&now) => holds(now >= *price, now, None),
            None => Level::Unknown,
        },
        Condition::PriceBelow {
            price_key, price, ..
        } => match ctx.prices.get(price_key) {
            Some(&now) => holds(now <= *price, now, None),
            None => Level::Unknown,
        },
        Condition::PortfolioChange { direction, ratio } => match ctx.portfolio_change {
            Some(change) => holds(moved(change, *direction, *ratio), change, None),
            None => Level::Unknown,
        },
        Condition::Depeg { ratio } => {
            let worst = ctx
                .stablecoins
                .iter()
                .map(|(symbol, price)| (symbol, *price, (*price - Decimal::ONE).abs()))
                .max_by(|a, b| a.2.cmp(&b.2));
            match worst {
                Some((symbol, price, gap)) => holds(gap >= *ratio, price, Some(symbol.clone())),
                None => Level::Unknown,
            }
        }
        Condition::BalanceChange { .. } => Level::Unknown,
    }
}

fn balance_triggers(
    rule: &Rule,
    wallet_id: Option<i64>,
    direction: Direction,
    usd: Decimal,
    ctx: &Context,
) -> Vec<Trigger> {
    let mut wallets: Vec<_> = ctx
        .flows
        .iter()
        .filter(|(id, _)| wallet_id.is_none_or(|w| w == **id))
        .collect();
    wallets.sort_by_key(|(id, _)| **id);
    wallets
        .into_iter()
        .filter_map(|(id, flow)| {
            let incoming =
                matches!(direction, Direction::Up | Direction::Either) && flow.incoming >= usd;
            let outgoing =
                matches!(direction, Direction::Down | Direction::Either) && flow.outgoing >= usd;
            // When both directions qualify, report the bigger move.
            let value = match (incoming, outgoing) {
                (true, true) if flow.outgoing > flow.incoming => -flow.outgoing,
                (true, _) => flow.incoming,
                (false, true) => -flow.outgoing,
                (false, false) => return None,
            };
            Some(Trigger {
                rule_id: rule.id,
                condition: rule.condition.clone(),
                value,
                subject: Some(id.to_string()),
            })
        })
        .collect()
}

fn moved(change: Decimal, direction: Direction, ratio: Decimal) -> bool {
    match direction {
        Direction::Up => change >= ratio,
        Direction::Down => change <= -ratio,
        Direction::Either => change.abs() >= ratio,
    }
}

/// Evaluates every rule after a sync. `before` is the balance snapshot taken
/// right before it; new armed states and triggers are saved.
pub fn check(
    conn: &mut Connection,
    before: &store::Snapshot,
    portfolio: &Portfolio,
) -> Result<Vec<Trigger>> {
    let after = store::snapshot(conn)?;
    let (prices, asset_prices) = store::prices(conn)?;
    let ctx = Context {
        prices,
        portfolio_change: portfolio.change_24h,
        flows: flows(before, &after, &asset_prices),
        stablecoins: portfolio
            .assets
            .iter()
            .filter(|a| is_stablecoin(&a.symbol) && a.value_usd >= Decimal::ONE)
            .map(|a| (a.symbol.clone(), a.price_usd))
            .collect(),
    };
    let outcomes = evaluate(&store::list_rules(conn)?, &ctx);
    store::save_outcomes(conn, &outcomes)?;
    Ok(outcomes.into_iter().flat_map(|o| o.triggers).collect())
}

/// Prices assets that price alerts watch but no wallet holds.
pub async fn price_watched(state: &AppState) -> Result<()> {
    let keys: Vec<String> = {
        let db = state.db()?;
        let held: HashSet<String> = repo::held_price_keys(&db)?.into_iter().collect();
        let mut keys: Vec<String> = store::list_rules(&db)?
            .iter()
            .filter(|rule| rule.enabled)
            .filter_map(|rule| rule.condition.price_key())
            .filter(|key| !held.contains(*key))
            .map(String::from)
            .collect();
        keys.sort();
        keys.dedup();
        keys
    };
    if keys.is_empty() {
        return Ok(());
    }
    let quotes = DefiLlama::new(state.http.clone()).quotes(&keys).await?;
    repo::save_sync(
        &mut *state.db()?,
        &SyncWrite {
            quotes,
            ..SyncWrite::default()
        },
    )
}

/// Symbols treated as USD stablecoins for depeg alerts.
pub const STABLECOINS: &[&str] = &[
    "USDT", "USDC", "DAI", "USDS", "USDE", "FDUSD", "PYUSD", "TUSD", "USDD", "RLUSD", "USDC.E",
    "USDT0", "GHO", "FRAX", "LUSD", "CRVUSD",
];

pub fn is_stablecoin(symbol: &str) -> bool {
    STABLECOINS.contains(&symbol.to_ascii_uppercase().as_str())
}

/// Per-wallet USD flows between two balance snapshots, valued at `prices`
/// (asset id -> USD). Wallets that had never synced are skipped so adding a
/// wallet doesn't look like a deposit.
pub fn flows(
    before: &store::Snapshot,
    after: &store::Snapshot,
    prices: &HashMap<String, Decimal>,
) -> HashMap<i64, Flow> {
    let mut flows: HashMap<i64, Flow> = HashMap::new();
    let keys = before.amounts.keys().chain(after.amounts.keys());
    let mut seen = HashSet::new();
    for key in keys {
        let (wallet_id, asset_id) = key;
        if !before.synced.contains(wallet_id) || !seen.insert(key) {
            continue;
        }
        let Some(price) = prices.get(asset_id) else {
            continue;
        };
        let old = before.amounts.get(key).copied().unwrap_or_default();
        let new = after.amounts.get(key).copied().unwrap_or_default();
        let value = (new - old) * price;
        let flow = flows.entry(*wallet_id).or_default();
        if value > Decimal::ZERO {
            flow.incoming += value;
        } else {
            flow.outgoing -= value;
        }
    }
    flows.retain(|_, f| *f != Flow::default());
    flows
}

#[cfg(test)]
mod tests {
    use super::{store::Snapshot, *};

    fn d(s: &str) -> Decimal {
        s.parse().unwrap()
    }

    fn rule(id: i64, condition: Condition) -> Rule {
        Rule {
            id,
            condition,
            enabled: true,
            armed: true,
            created_at: 0,
            last_triggered_at: None,
        }
    }

    fn btc_above(price: Decimal) -> Condition {
        Condition::PriceAbove {
            price_key: "coingecko:bitcoin".into(),
            symbol: "BTC".into(),
            price,
        }
    }

    fn priced(btc: Decimal) -> Context {
        Context {
            prices: HashMap::from([("coingecko:bitcoin".into(), btc)]),
            ..Context::default()
        }
    }

    #[test]
    fn price_alert_fires_once_per_crossing() {
        let mut r = rule(1, btc_above(d("100000")));

        let out = evaluate(std::slice::from_ref(&r), &priced(d("99000")));
        assert!(out[0].triggers.is_empty());
        assert!(out[0].armed);

        let out = evaluate(std::slice::from_ref(&r), &priced(d("101000")));
        assert_eq!(out[0].triggers.len(), 1);
        assert_eq!(out[0].triggers[0].value, d("101000"));
        r.armed = out[0].armed;
        assert!(!r.armed);

        // Still above: no repeat notification.
        let out = evaluate(std::slice::from_ref(&r), &priced(d("102000")));
        assert!(out[0].triggers.is_empty());

        // Back below re-arms; crossing again fires again.
        r.armed = evaluate(std::slice::from_ref(&r), &priced(d("98000")))[0].armed;
        assert!(r.armed);
        assert_eq!(evaluate(&[r], &priced(d("100000")))[0].triggers.len(), 1);
    }

    #[test]
    fn price_below_and_missing_prices() {
        let mut r = rule(
            2,
            Condition::PriceBelow {
                price_key: "coingecko:ethereum".into(),
                symbol: "ETH".into(),
                price: d("2000"),
            },
        );
        r.armed = false;
        // No price for ETH: state is kept as is.
        let out = evaluate(std::slice::from_ref(&r), &priced(d("1")));
        assert!(!out[0].armed);
        assert!(out[0].triggers.is_empty());

        let ctx = Context {
            prices: HashMap::from([("coingecko:ethereum".into(), d("1999.5"))]),
            ..Context::default()
        };
        r.armed = true;
        assert_eq!(evaluate(&[r], &ctx)[0].triggers[0].value, d("1999.5"));
    }

    #[test]
    fn disabled_rules_are_skipped() {
        let mut r = rule(3, btc_above(d("1")));
        r.enabled = false;
        assert!(evaluate(&[r], &priced(d("5"))).is_empty());
    }

    #[test]
    fn portfolio_change_respects_direction() {
        let ctx = |change| Context {
            portfolio_change: Some(change),
            ..Context::default()
        };
        let down = rule(
            4,
            Condition::PortfolioChange {
                direction: Direction::Down,
                ratio: d("0.1"),
            },
        );
        let either = rule(
            5,
            Condition::PortfolioChange {
                direction: Direction::Either,
                ratio: d("0.1"),
            },
        );
        let rules = [down, either];

        let out = evaluate(&rules, &ctx(d("0.12")));
        assert!(out[0].triggers.is_empty());
        assert_eq!(out[1].triggers[0].value, d("0.12"));

        let out = evaluate(&rules, &ctx(d("-0.1")));
        assert_eq!(out[0].triggers.len(), 1);
        assert_eq!(out[1].triggers.len(), 1);

        let out = evaluate(&rules, &ctx(d("-0.05")));
        assert!(out.iter().all(|o| o.triggers.is_empty() && o.armed));
    }

    #[test]
    fn depeg_reports_the_worst_stablecoin() {
        let ctx = Context {
            stablecoins: vec![
                ("USDC".into(), d("0.999")),
                ("USDT".into(), d("0.975")),
                ("DAI".into(), d("1.012")),
            ],
            ..Context::default()
        };
        let out = evaluate(&[rule(6, Condition::Depeg { ratio: d("0.02") })], &ctx);
        let trigger = &out[0].triggers[0];
        assert_eq!(trigger.subject.as_deref(), Some("USDT"));
        assert_eq!(trigger.value, d("0.975"));

        let out = evaluate(&[rule(7, Condition::Depeg { ratio: d("0.03") })], &ctx);
        assert!(out[0].triggers.is_empty());
        assert!(out[0].armed);
        assert!(is_stablecoin("usdc"));
        assert!(!is_stablecoin("ETH"));
    }

    #[test]
    fn balance_change_fires_every_time_and_filters_wallets() {
        let ctx = Context {
            flows: HashMap::from([
                (
                    1,
                    Flow {
                        incoming: d("5000"),
                        outgoing: Decimal::ZERO,
                    },
                ),
                (
                    2,
                    Flow {
                        incoming: d("100"),
                        outgoing: d("8000"),
                    },
                ),
            ]),
            ..Context::default()
        };
        let any = rule(
            8,
            Condition::BalanceChange {
                wallet_id: None,
                direction: Direction::Either,
                usd: d("1000"),
            },
        );
        let out = evaluate(std::slice::from_ref(&any), &ctx);
        let values: Vec<_> = out[0].triggers.iter().map(|t| t.value).collect();
        assert_eq!(values, vec![d("5000"), d("-8000")]);
        assert!(out[0].armed);
        // Same flows next sync fire again: they are events, not levels.
        assert_eq!(evaluate(&[any], &ctx)[0].triggers.len(), 2);

        let incoming_to_2 = rule(
            9,
            Condition::BalanceChange {
                wallet_id: Some(2),
                direction: Direction::Up,
                usd: d("1000"),
            },
        );
        assert!(evaluate(&[incoming_to_2], &ctx)[0].triggers.is_empty());
    }

    #[test]
    fn flows_value_balance_deltas() {
        let snapshot = |amounts: &[(i64, &str, Decimal)], synced: &[i64]| Snapshot {
            amounts: amounts
                .iter()
                .map(|(w, a, amount)| ((*w, a.to_string()), *amount))
                .collect(),
            synced: synced.iter().copied().collect::<HashSet<_>>(),
        };
        let before = snapshot(
            &[
                (1, "ethereum:native", d("2")),
                (1, "ethereum:usdc", d("500")),
            ],
            &[1],
        );
        let after = snapshot(
            &[
                (1, "ethereum:native", d("1.5")),
                (1, "ethereum:usdc", d("1500")),
                (1, "ethereum:spam", d("1000000")),
                // Wallet 2 was just added: its first balances aren't a deposit.
                (2, "ethereum:native", d("10")),
            ],
            &[1, 2],
        );
        let prices = HashMap::from([
            ("ethereum:native".to_string(), d("3000")),
            ("ethereum:usdc".to_string(), d("1")),
        ]);
        let flows = flows(&before, &after, &prices);
        assert_eq!(flows.len(), 1);
        assert_eq!(
            flows[&1],
            Flow {
                incoming: d("1000"),
                outgoing: d("1500"),
            }
        );
    }

    #[test]
    fn check_evaluates_rules_against_a_real_sync() {
        use crate::{
            chains::{AssetMeta, Chain, Family, Holding},
            db::{
                open_in_memory,
                repo::{ChainBalances, NewWallet, WalletKind, insert_wallet, portfolio, save_sync},
            },
            prices::Quote,
        };

        let mut conn = open_in_memory().unwrap();
        let wallet = insert_wallet(
            &mut conn,
            &NewWallet {
                label: "Hot",
                family: Family::Evm,
                kind: WalletKind::Address,
                address: "0xa",
                input: "0xa",
            },
        )
        .unwrap();
        let eth = AssetMeta::native(Chain::Ethereum, "ETH", "Ether", 18, "ethereum");
        let usdt = AssetMeta::token(Chain::Ethereum, "0xusdt", 6)
            .with_names(Some("USDT".into()), Some("Tether".into()));
        let sync = |conn: &mut Connection, eth_amount: &str, usdt_price: &str| {
            let mut write = SyncWrite::default();
            write.balances.push(ChainBalances {
                wallet_id: wallet.id,
                chain: Chain::Ethereum,
                holdings: vec![
                    Holding {
                        asset: eth.clone(),
                        amount: d(eth_amount),
                        locked: None,
                    },
                    Holding {
                        asset: usdt.clone(),
                        amount: d("1000"),
                        locked: None,
                    },
                ],
            });
            let quote = |price: &str| Quote {
                price_usd: d(price),
                change_24h: None,
                symbol: None,
            };
            write
                .quotes
                .insert("coingecko:ethereum".into(), quote("3000"));
            write
                .quotes
                .insert("ethereum:0xusdt".into(), quote(usdt_price));
            write.synced_wallets.insert(wallet.id);
            save_sync(conn, &write).unwrap();
        };

        let incoming = store::insert_rule(
            &conn,
            &Condition::BalanceChange {
                wallet_id: Some(wallet.id),
                direction: Direction::Up,
                usd: d("1000"),
            },
            true,
        )
        .unwrap();
        let depeg =
            store::insert_rule(&conn, &Condition::Depeg { ratio: d("0.01") }, true).unwrap();

        // First sync of a new wallet: nothing moved "in", USDT is on peg.
        let before = store::snapshot(&conn).unwrap();
        sync(&mut conn, "1", "1");
        let p = portfolio(&conn).unwrap();
        assert!(check(&mut conn, &before, &p).unwrap().is_empty());

        // 1 ETH arrives and USDT slips to 0.98.
        let before = store::snapshot(&conn).unwrap();
        sync(&mut conn, "2", "0.98");
        let p = portfolio(&conn).unwrap();
        let triggers = check(&mut conn, &before, &p).unwrap();
        assert_eq!(triggers.len(), 2);
        assert_eq!(triggers[0].rule_id, incoming);
        assert_eq!(triggers[0].value, d("3000"));
        assert_eq!(triggers[1].rule_id, depeg);
        assert_eq!(triggers[1].subject.as_deref(), Some("USDT"));
        assert_eq!(store::recent_events(&conn, 10).unwrap().len(), 2);

        // Nothing changes: the depeg already notified and stays quiet.
        let before = store::snapshot(&conn).unwrap();
        sync(&mut conn, "2", "0.98");
        let p = portfolio(&conn).unwrap();
        assert!(check(&mut conn, &before, &p).unwrap().is_empty());
    }

    #[test]
    fn conditions_round_trip_as_tagged_json() {
        let json = serde_json::to_string(&Condition::BalanceChange {
            wallet_id: Some(3),
            direction: Direction::Down,
            usd: d("250"),
        })
        .unwrap();
        assert_eq!(
            json,
            r#"{"kind":"balanceChange","walletId":3,"direction":"down","usd":"250"}"#
        );
        assert!(serde_json::from_str::<Condition>(&json).unwrap().validate());
        assert!(!Condition::Depeg { ratio: d("1.5") }.validate());
    }
}
