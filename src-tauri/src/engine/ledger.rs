//! Cost-basis lots per asset: FIFO, LIFO or weighted average.
//!
//! Values are in whatever currency the caller priced the events in, so the
//! same ledger serves USD analytics and BRL tax reports.

use std::collections::{HashMap, VecDeque};

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Method {
    #[default]
    Fifo,
    Lifo,
    Average,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    /// Bought, swapped into, received or earned. `cost` is `None` when no
    /// price was known; the lot then has a zero, estimated cost.
    Acquire {
        amount: Decimal,
        cost: Option<Decimal>,
        income: bool,
    },
    /// Sold or swapped away: realizes a gain against the lots it consumes.
    Dispose {
        amount: Decimal,
        proceeds: Option<Decimal>,
    },
    /// Left the portfolio without a sale (sent away, gifted): its cost basis
    /// leaves with it and nothing is realized.
    Withdraw { amount: Decimal },
    /// Network fee: consumed at cost, a realized loss.
    Fee {
        amount: Decimal,
        value: Option<Decimal>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Event {
    pub t: i64,
    pub tx_id: String,
    /// Asset pool (a price key: the same coin on several chains shares lots).
    pub pool: String,
    pub action: Action,
}

#[derive(Clone, Debug, PartialEq)]
struct Lot {
    amount: Decimal,
    cost: Decimal,
}

/// Running state of one asset.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Pool {
    lots: VecDeque<Lot>,
    pub realized: Decimal,
    pub income: Decimal,
    /// Market value of the fees paid in this asset.
    pub fees: Decimal,
    pub proceeds: Decimal,
    /// Cost basis of everything acquired, ever.
    pub acquired_cost: Decimal,
    /// Some cost came from a missing price or more was spent than tracked.
    pub estimated: bool,
}

impl Pool {
    pub fn amount(&self) -> Decimal {
        self.lots.iter().map(|l| l.amount).sum()
    }

    pub fn cost(&self) -> Decimal {
        self.lots.iter().map(|l| l.cost).sum()
    }

    fn add(&mut self, method: Method, amount: Decimal, cost: Decimal) {
        if method == Method::Average {
            if let Some(lot) = self.lots.front_mut() {
                lot.amount += amount;
                lot.cost += cost;
                return;
            }
        }
        self.lots.push_back(Lot { amount, cost });
    }

    /// Removes `amount` from the lots and returns its cost basis.
    fn take(&mut self, method: Method, mut amount: Decimal) -> Decimal {
        let mut cost = Decimal::ZERO;
        while amount > Decimal::ZERO {
            let lot = match method {
                Method::Lifo => self.lots.back_mut(),
                Method::Fifo | Method::Average => self.lots.front_mut(),
            };
            let Some(lot) = lot else {
                // Spent more than tracked (history older than the sync).
                self.estimated = true;
                break;
            };
            if lot.amount <= amount {
                amount -= lot.amount;
                cost += lot.cost;
                match method {
                    Method::Lifo => self.lots.pop_back(),
                    Method::Fifo | Method::Average => self.lots.pop_front(),
                };
            } else {
                let part = lot.cost * amount / lot.amount;
                lot.amount -= amount;
                lot.cost -= part;
                cost += part;
                amount = Decimal::ZERO;
            }
        }
        cost
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Disposal {
    pub t: i64,
    pub tx_id: String,
    pub pool: String,
    pub amount: Decimal,
    pub proceeds: Decimal,
    pub cost: Decimal,
    pub gain: Decimal,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Ledger {
    pub pools: HashMap<String, Pool>,
    pub disposals: Vec<Disposal>,
}

/// Replays `events` (any order; sorted by time here) into lots.
pub fn run(method: Method, events: &[Event]) -> Ledger {
    let mut ordered: Vec<&Event> = events.iter().collect();
    ordered.sort_by_key(|e| e.t);
    let mut ledger = Ledger::default();
    for event in ordered {
        let pool = ledger.pools.entry(event.pool.clone()).or_default();
        match &event.action {
            Action::Acquire {
                amount,
                cost,
                income,
            } => {
                let value = cost.unwrap_or(Decimal::ZERO);
                pool.estimated |= cost.is_none();
                pool.acquired_cost += value;
                if *income {
                    pool.income += value;
                }
                pool.add(method, *amount, value);
            }
            Action::Dispose { amount, proceeds } => {
                let cost = pool.take(method, *amount);
                let proceeds = proceeds.unwrap_or_else(|| {
                    pool.estimated = true;
                    cost
                });
                pool.realized += proceeds - cost;
                pool.proceeds += proceeds;
                ledger.disposals.push(Disposal {
                    t: event.t,
                    tx_id: event.tx_id.clone(),
                    pool: event.pool.clone(),
                    amount: *amount,
                    proceeds,
                    cost,
                    gain: proceeds - cost,
                });
            }
            Action::Withdraw { amount } => {
                pool.take(method, *amount);
            }
            Action::Fee { amount, value } => {
                let cost = pool.take(method, *amount);
                pool.realized -= cost;
                pool.fees += value.unwrap_or(cost);
            }
        }
    }
    ledger
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> Decimal {
        s.parse().unwrap()
    }

    fn ev(t: i64, action: Action) -> Event {
        Event {
            t,
            tx_id: format!("tx{t}"),
            pool: "eth".into(),
            action,
        }
    }

    fn buy(t: i64, amount: &str, cost: &str) -> Event {
        ev(
            t,
            Action::Acquire {
                amount: d(amount),
                cost: Some(d(cost)),
                income: false,
            },
        )
    }

    fn sell(t: i64, amount: &str, proceeds: &str) -> Event {
        ev(
            t,
            Action::Dispose {
                amount: d(amount),
                proceeds: Some(d(proceeds)),
            },
        )
    }

    fn history() -> Vec<Event> {
        vec![buy(1, "1", "100"), buy(2, "1", "300"), sell(3, "1", "250")]
    }

    #[test]
    fn fifo_sells_oldest_first() {
        let ledger = run(Method::Fifo, &history());
        let pool = &ledger.pools["eth"];
        assert_eq!(pool.realized, d("150"));
        assert_eq!(pool.cost(), d("300"));
        assert_eq!(ledger.disposals[0].cost, d("100"));
    }

    #[test]
    fn lifo_sells_newest_first() {
        let ledger = run(Method::Lifo, &history());
        let pool = &ledger.pools["eth"];
        assert_eq!(pool.realized, d("-50"));
        assert_eq!(pool.cost(), d("100"));
    }

    #[test]
    fn average_blends_cost() {
        let ledger = run(Method::Average, &history());
        let pool = &ledger.pools["eth"];
        assert_eq!(pool.realized, d("50"));
        assert_eq!(pool.cost(), d("200"));
        assert_eq!(pool.amount(), d("1"));
    }

    #[test]
    fn partial_lots_split_cost_pro_rata() {
        let events = vec![buy(1, "4", "400"), sell(2, "1", "150")];
        let ledger = run(Method::Fifo, &events);
        let pool = &ledger.pools["eth"];
        assert_eq!(pool.realized, d("50"));
        assert_eq!(pool.amount(), d("3"));
        assert_eq!(pool.cost(), d("300"));
    }

    #[test]
    fn income_withdrawals_and_fees() {
        let events = vec![
            ev(
                1,
                Action::Acquire {
                    amount: d("2"),
                    cost: Some(d("200")),
                    income: true,
                },
            ),
            ev(2, Action::Withdraw { amount: d("1") }),
            ev(
                3,
                Action::Fee {
                    amount: d("0.1"),
                    value: Some(d("15")),
                },
            ),
        ];
        let ledger = run(Method::Fifo, &events);
        let pool = &ledger.pools["eth"];
        assert_eq!(pool.income, d("200"));
        // The withdrawal realizes nothing; the fee costs its basis.
        assert_eq!(pool.realized, d("-10"));
        assert_eq!(pool.fees, d("15"));
        assert_eq!(pool.amount(), d("0.9"));
        assert!(ledger.disposals.is_empty());
    }

    #[test]
    fn overspending_and_missing_prices_are_estimated() {
        let events = vec![buy(1, "1", "100"), sell(2, "2", "500")];
        let ledger = run(Method::Fifo, &events);
        let pool = &ledger.pools["eth"];
        assert!(pool.estimated);
        assert_eq!(pool.realized, d("400"));

        let unpriced = vec![ev(
            1,
            Action::Acquire {
                amount: d("1"),
                cost: None,
                income: false,
            },
        )];
        assert!(run(Method::Fifo, &unpriced).pools["eth"].estimated);
    }

    #[test]
    fn events_are_sorted_by_time() {
        let mut events = history();
        events.reverse();
        assert_eq!(run(Method::Fifo, &events), run(Method::Fifo, &history()));
    }
}
