//! Turns stored transactions into ledger events and reconciles them with
//! current balances.

use std::collections::{BTreeMap, HashMap, HashSet};

use rust_decimal::Decimal;

use super::ledger::{self, Action, Event, Ledger, Method};
use crate::history::classify::Kind;

/// What one transaction did to one wallet, in asset pools (price keys).
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub tx_id: String,
    pub t: i64,
    pub kind: Kind,
    /// The user picked `kind`; it wins over other wallets' automatic kinds.
    pub overridden: bool,
    /// Transfers per pool, fee excluded (positive in, negative out).
    pub deltas: Vec<(String, Decimal)>,
    pub fee: Option<(String, Decimal)>,
}

/// Price of one unit of `pool` at a Unix ms time, in the report currency.
pub trait Prices {
    fn at(&self, pool: &str, t: i64) -> Option<Decimal>;
}

impl<F: Fn(&str, i64) -> Option<Decimal>> Prices for F {
    fn at(&self, pool: &str, t: i64) -> Option<Decimal> {
        self(pool, t)
    }
}

/// Tx id of the synthetic lot that holds what was owned before the oldest
/// synced transaction.
pub const OPENING: &str = "opening";

pub struct Outcome {
    pub ledger: Ledger,
    /// Pools whose cost basis partly comes from an opening lot or a
    /// reconciliation, i.e. from history the app hasn't seen.
    pub reconciled: HashSet<String>,
}

/// Replays every entry, then reconciles each pool with `holdings` (current
/// amount per pool) so the remaining lots match what is actually held.
pub fn compute(
    method: Method,
    entries: &[Entry],
    holdings: &HashMap<String, Decimal>,
    prices: &impl Prices,
    now: i64,
) -> Outcome {
    let mut events = to_events(entries, prices);
    let mut net: HashMap<&str, Decimal> = HashMap::new();
    let mut first_seen: HashMap<&str, i64> = HashMap::new();
    for e in &events {
        let delta = match &e.action {
            Action::Acquire { amount, .. } => *amount,
            Action::Dispose { amount, .. }
            | Action::Withdraw { amount }
            | Action::Fee { amount, .. } => -*amount,
        };
        *net.entry(&e.pool).or_default() += delta;
        let seen = first_seen.entry(&e.pool).or_insert(e.t);
        *seen = (*seen).min(e.t);
    }

    let pools: HashSet<&str> = holdings
        .keys()
        .map(String::as_str)
        .chain(net.keys().copied())
        .collect();
    let mut extra = Vec::new();
    let mut reconciled = HashSet::new();
    for pool in pools {
        let held = holdings.get(pool).copied().unwrap_or_default();
        let gap = held - net.get(pool).copied().unwrap_or_default();
        if is_dust(gap, held) {
            continue;
        }
        reconciled.insert(pool.to_string());
        if gap > Decimal::ZERO {
            // Held before the oldest synced transaction: valued on the first
            // day the app has seen it.
            let t = first_seen.get(pool).map_or(now, |t| t - 1);
            extra.push(Event {
                t,
                tx_id: OPENING.into(),
                pool: pool.to_string(),
                action: Action::Acquire {
                    amount: gap,
                    cost: prices.at(pool, t).map(|p| p * gap),
                    income: false,
                },
            });
        } else {
            // Left through something the history doesn't show (a bridge, an
            // unsupported chain): drop it at cost.
            extra.push(Event {
                t: now,
                tx_id: OPENING.into(),
                pool: pool.to_string(),
                action: Action::Withdraw { amount: -gap },
            });
        }
    }
    events.extend(extra);
    Outcome {
        ledger: ledger::run(method, &events),
        reconciled,
    }
}

fn is_dust(gap: Decimal, held: Decimal) -> bool {
    let tolerance = (held.abs() * Decimal::new(1, 6)).max(Decimal::new(1, 12));
    gap.abs() <= tolerance
}

/// Ledger events for every transaction, from the point of view of the whole
/// portfolio (transfers between own wallets cancel out).
pub fn to_events(entries: &[Entry], prices: &impl Prices) -> Vec<Event> {
    let mut by_tx: BTreeMap<&str, Vec<&Entry>> = BTreeMap::new();
    for entry in entries {
        by_tx.entry(&entry.tx_id).or_default().push(entry);
    }
    let mut events = Vec::new();
    for (tx_id, group) in by_tx {
        let t = group[0].t;
        let event = |pool: &str, action| Event {
            t,
            tx_id: tx_id.to_string(),
            pool: pool.to_string(),
            action,
        };

        for (pool, amount) in group.iter().filter_map(|e| e.fee.as_ref()) {
            if *amount > Decimal::ZERO {
                let value = prices.at(pool, t).map(|p| p * amount);
                events.push(event(
                    pool,
                    Action::Fee {
                        amount: *amount,
                        value,
                    },
                ));
            }
        }

        let kind = tx_kind(&group);
        if matches!(kind, Kind::Spam | Kind::Failed) {
            continue;
        }
        let mut net: BTreeMap<&str, Decimal> = BTreeMap::new();
        for (pool, amount) in group.iter().flat_map(|e| &e.deltas) {
            *net.entry(pool).or_default() += amount;
        }
        let ins: Vec<(&str, Decimal)> = net
            .iter()
            .filter(|(_, a)| **a > Decimal::ZERO)
            .map(|(p, a)| (*p, *a))
            .collect();
        let outs: Vec<(&str, Decimal)> = net
            .iter()
            .filter(|(_, a)| **a < Decimal::ZERO)
            .map(|(p, a)| (*p, -*a))
            .collect();
        let market = |pool: &str, amount: Decimal| prices.at(pool, t).map(|p| p * amount);

        if kind == Kind::Swap && !ins.is_empty() && !outs.is_empty() {
            // Proceeds of what left = cost of what arrived = the market value
            // of the received side (or of the sent side when that's unknown).
            let in_values = values(&ins, &market);
            let out_values = values(&outs, &market);
            let total = total(&in_values).or_else(|| total(&out_values));
            for ((pool, amount), share) in outs.iter().zip(shares(&out_values)) {
                let proceeds = total.map(|v| v * share);
                events.push(event(
                    pool,
                    Action::Dispose {
                        amount: *amount,
                        proceeds,
                    },
                ));
            }
            for ((pool, amount), share) in ins.iter().zip(shares(&in_values)) {
                let cost = total.map(|v| v * share);
                events.push(event(
                    pool,
                    Action::Acquire {
                        amount: *amount,
                        cost,
                        income: false,
                    },
                ));
            }
            continue;
        }

        let income = matches!(kind, Kind::Income | Kind::Reward | Kind::Airdrop);
        for (pool, amount) in &outs {
            let action = if matches!(kind, Kind::Sale | Kind::Swap) {
                Action::Dispose {
                    amount: *amount,
                    proceeds: market(pool, *amount),
                }
            } else {
                Action::Withdraw { amount: *amount }
            };
            events.push(event(pool, action));
        }
        for (pool, amount) in &ins {
            events.push(event(
                pool,
                Action::Acquire {
                    amount: *amount,
                    cost: market(pool, *amount),
                    income,
                },
            ));
        }
    }
    events
}

/// One kind for the whole transaction: a user's choice first, then a swap
/// seen by any wallet, then the first wallet's view that isn't internal.
fn tx_kind(group: &[&Entry]) -> Kind {
    if let Some(e) = group.iter().find(|e| e.overridden) {
        return e.kind;
    }
    if group.iter().any(|e| e.kind == Kind::Swap) {
        return Kind::Swap;
    }
    group
        .iter()
        .map(|e| e.kind)
        .find(|k| *k != Kind::Internal)
        .unwrap_or(Kind::Internal)
}

fn values(
    side: &[(&str, Decimal)],
    market: &impl Fn(&str, Decimal) -> Option<Decimal>,
) -> Vec<Option<Decimal>> {
    side.iter().map(|(p, a)| market(p, *a)).collect()
}

fn total(values: &[Option<Decimal>]) -> Option<Decimal> {
    values.iter().copied().sum()
}

/// Each leg's share of a side, by market value when every leg is priced.
fn shares(values: &[Option<Decimal>]) -> Vec<Decimal> {
    match total(values) {
        Some(sum) if sum > Decimal::ZERO => {
            values.iter().map(|v| v.unwrap_or_default() / sum).collect()
        }
        _ => {
            let n = Decimal::from(values.len().max(1));
            values.iter().map(|_| Decimal::ONE / n).collect()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> Decimal {
        s.parse().unwrap()
    }

    fn prices(pool: &str, t: i64) -> Option<Decimal> {
        match (pool, t) {
            ("eth", t) if t < 100 => Some(d("1000")),
            ("eth", _) => Some(d("2000")),
            ("usdc", _) => Some(Decimal::ONE),
            _ => None,
        }
    }

    fn entry(tx: &str, t: i64, kind: Kind, deltas: &[(&str, &str)]) -> Entry {
        Entry {
            tx_id: tx.into(),
            t,
            kind,
            overridden: false,
            deltas: deltas.iter().map(|(p, a)| (p.to_string(), d(a))).collect(),
            fee: None,
        }
    }

    fn holdings(items: &[(&str, &str)]) -> HashMap<String, Decimal> {
        items.iter().map(|(p, a)| (p.to_string(), d(a))).collect()
    }

    #[test]
    fn receive_then_swap_realizes_gain() {
        let entries = vec![
            entry("a", 10, Kind::Receive, &[("eth", "2")]),
            entry("b", 200, Kind::Swap, &[("eth", "-1"), ("usdc", "2000")]),
        ];
        let out = compute(
            Method::Fifo,
            &entries,
            &holdings(&[("eth", "1"), ("usdc", "2000")]),
            &prices,
            300,
        );
        let eth = &out.ledger.pools["eth"];
        assert_eq!(eth.realized, d("1000"));
        assert_eq!(eth.cost(), d("1000"));
        assert_eq!(out.ledger.pools["usdc"].cost(), d("2000"));
        assert!(out.reconciled.is_empty());
    }

    #[test]
    fn internal_transfers_cancel_and_fees_count() {
        let mut send = entry("x", 10, Kind::Internal, &[("eth", "-1")]);
        send.fee = Some(("eth".into(), d("0.01")));
        let receive = entry("x", 10, Kind::Internal, &[("eth", "1")]);
        let entries = vec![entry("a", 5, Kind::Receive, &[("eth", "1")]), send, receive];
        let out = compute(
            Method::Fifo,
            &entries,
            &holdings(&[("eth", "0.99")]),
            &prices,
            300,
        );
        let eth = &out.ledger.pools["eth"];
        assert!(out.ledger.disposals.is_empty());
        assert_eq!(eth.amount(), d("0.99"));
        assert_eq!(eth.fees, d("10"));
        assert_eq!(eth.realized, d("-10"));
    }

    #[test]
    fn override_turns_a_send_into_a_sale_and_income() {
        let mut sale = entry("s", 200, Kind::Sale, &[("eth", "-1")]);
        sale.overridden = true;
        let mut salary = entry("i", 10, Kind::Income, &[("eth", "2")]);
        salary.overridden = true;
        let out = compute(
            Method::Fifo,
            &[salary, sale],
            &holdings(&[("eth", "1")]),
            &prices,
            300,
        );
        let eth = &out.ledger.pools["eth"];
        assert_eq!(eth.income, d("2000"));
        assert_eq!(eth.realized, d("1000"));
        assert_eq!(out.ledger.disposals.len(), 1);
    }

    #[test]
    fn spam_and_failed_only_keep_fees() {
        let mut failed = entry("f", 10, Kind::Failed, &[]);
        failed.fee = Some(("eth".into(), d("0.1")));
        let spam = entry("s", 20, Kind::Spam, &[("eth", "5")]);
        let events = to_events(&[failed, spam], &prices);
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0].action, Action::Fee { .. }));
    }

    #[test]
    fn unseen_history_becomes_an_opening_lot() {
        let entries = vec![entry("b", 200, Kind::Send, &[("eth", "-1")])];
        let out = compute(
            Method::Fifo,
            &entries,
            &holdings(&[("eth", "2"), ("usdc", "50")]),
            &prices,
            300,
        );
        // 3 ETH were held before the first synced tx, valued that day.
        let eth = &out.ledger.pools["eth"];
        assert_eq!(eth.amount(), d("2"));
        assert_eq!(eth.cost(), d("4000"));
        // A holding with no history at all costs its current value.
        assert_eq!(out.ledger.pools["usdc"].cost(), d("50"));
        assert!(out.reconciled.contains("eth") && out.reconciled.contains("usdc"));
    }

    #[test]
    fn missing_outflows_are_withdrawn_at_cost() {
        let entries = vec![entry("a", 10, Kind::Receive, &[("eth", "3")])];
        let out = compute(
            Method::Fifo,
            &entries,
            &holdings(&[("eth", "1")]),
            &prices,
            300,
        );
        let eth = &out.ledger.pools["eth"];
        assert_eq!(eth.amount(), d("1"));
        assert_eq!(eth.cost(), d("1000"));
        assert_eq!(eth.realized, Decimal::ZERO);
    }

    #[test]
    fn swap_into_unpriced_token_uses_what_was_given() {
        let entries = vec![
            entry("a", 10, Kind::Receive, &[("usdc", "100")]),
            entry("b", 20, Kind::Swap, &[("usdc", "-100"), ("new", "5000")]),
        ];
        let events = to_events(&entries, &prices);
        let acquire = events
            .iter()
            .find(|e| e.pool == "new")
            .map(|e| e.action.clone())
            .unwrap();
        assert_eq!(
            acquire,
            Action::Acquire {
                amount: d("5000"),
                cost: Some(d("100")),
                income: false
            }
        );
    }
}
