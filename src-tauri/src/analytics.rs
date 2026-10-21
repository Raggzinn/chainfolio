//! Profit and loss across the whole portfolio, from stored history.

use std::collections::HashMap;

use rusqlite::Connection;
use rust_decimal::Decimal;
use serde::Serialize;

use crate::{
    db::{history::TxAsset, ledger as store},
    engine::{
        ledger::Method,
        pnl::{self, OPENING},
        replay::price_at,
    },
    error::Result,
};

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PnlTotals {
    pub value: Decimal,
    pub cost_basis: Decimal,
    pub unrealized: Decimal,
    pub realized: Decimal,
    pub income: Decimal,
    pub fees: Decimal,
    /// Realized + unrealized.
    pub pnl: Decimal,
    /// `pnl` over everything ever paid for the assets involved.
    pub pnl_pct: Option<Decimal>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PnlAsset {
    pub price_key: String,
    pub asset: TxAsset,
    pub amount: Decimal,
    pub price: Decimal,
    pub value: Decimal,
    pub cost_basis: Decimal,
    pub avg_cost: Option<Decimal>,
    pub unrealized: Decimal,
    pub unrealized_pct: Option<Decimal>,
    pub realized: Decimal,
    pub income: Decimal,
    pub fees: Decimal,
    /// Part of the cost basis is estimated (history older than the sync, or a
    /// missing price).
    pub estimated: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PnlDisposal {
    pub t: i64,
    pub tx_id: String,
    pub asset: TxAsset,
    pub amount: Decimal,
    pub proceeds: Decimal,
    pub cost: Decimal,
    pub gain: Decimal,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PnlReport {
    pub method: Method,
    pub totals: PnlTotals,
    pub assets: Vec<PnlAsset>,
    /// Newest first.
    pub disposals: Vec<PnlDisposal>,
    /// Every wallet's history is fully synced.
    pub complete: bool,
    pub oldest_transaction: Option<i64>,
}

fn pct(part: Decimal, whole: Decimal) -> Option<Decimal> {
    (whole > Decimal::ZERO).then(|| (part / whole * Decimal::ONE_HUNDRED).round_dp(4))
}

pub fn pnl_report(conn: &Connection, method: Method, now: i64) -> Result<PnlReport> {
    let entries = store::entries(conn)?;
    let holdings = store::pool_holdings(conn)?;
    let current = store::current_prices(conn)?;
    let assets = store::pool_assets(conn)?;
    let mut keys: Vec<String> = current.keys().cloned().collect();
    keys.sort();
    let series = store::price_series(conn, &keys)?;
    let prices = |pool: &str, t: i64| {
        series
            .get(pool)
            .and_then(|s| price_at(s, t))
            .or_else(|| current.get(pool).copied())
    };

    let amounts: HashMap<String, Decimal> =
        holdings.iter().map(|(k, (a, _))| (k.clone(), *a)).collect();
    let outcome = pnl::compute(method, &entries, &amounts, &prices, now);

    let mut totals = PnlTotals::default();
    let mut rows = Vec::new();
    let mut disposed_cost = Decimal::ZERO;
    for (key, pool) in &outcome.ledger.pools {
        let Some(asset) = assets.get(key) else {
            continue;
        };
        let amount = pool.amount();
        if amount.is_zero() && pool.realized.is_zero() && pool.income.is_zero() {
            continue;
        }
        let price = current.get(key).copied().unwrap_or_default();
        let value = amount * price;
        let cost_basis = pool.cost();
        let unrealized = value - cost_basis;
        totals.value += value;
        totals.cost_basis += cost_basis;
        totals.unrealized += unrealized;
        totals.realized += pool.realized;
        totals.income += pool.income;
        totals.fees += pool.fees;
        rows.push(PnlAsset {
            price_key: key.clone(),
            asset: asset.clone(),
            amount,
            price,
            value,
            cost_basis,
            avg_cost: (amount > Decimal::ZERO).then(|| cost_basis / amount),
            unrealized,
            unrealized_pct: pct(unrealized, cost_basis),
            realized: pool.realized,
            income: pool.income,
            fees: pool.fees,
            estimated: pool.estimated || outcome.reconciled.contains(key),
        });
    }
    rows.sort_by(|a, b| b.value.cmp(&a.value).then(b.realized.cmp(&a.realized)));

    let mut disposals: Vec<PnlDisposal> = outcome
        .ledger
        .disposals
        .iter()
        .filter(|d| d.tx_id != OPENING)
        .filter_map(|d| {
            disposed_cost += d.cost;
            Some(PnlDisposal {
                t: d.t,
                tx_id: d.tx_id.clone(),
                asset: assets.get(&d.pool)?.clone(),
                amount: d.amount,
                proceeds: d.proceeds,
                cost: d.cost,
                gain: d.gain,
            })
        })
        .collect();
    disposals.sort_by_key(|d| std::cmp::Reverse(d.t));

    totals.pnl = totals.realized + totals.unrealized;
    totals.pnl_pct = pct(totals.pnl, totals.cost_basis + disposed_cost);
    let (complete, oldest_transaction) = store::coverage(conn)?;
    Ok(PnlReport {
        method,
        totals,
        assets: rows,
        disposals,
        complete,
        oldest_transaction,
    })
}
