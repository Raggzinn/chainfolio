//! Rebuilds past holdings by replaying transactions backwards from today's
//! balances, then values them with historical prices. Anchoring on current
//! balances keeps the latest point exact even when old history is missing.

use std::collections::HashMap;

use rust_decimal::Decimal;

/// Holdings per asset id at each of `points` (Unix ms, ascending).
/// `now` are today's balances; `legs` are `(timestamp, asset_id, net change)`.
pub fn holdings_at(
    now: &HashMap<String, Decimal>,
    legs: &[(i64, String, Decimal)],
    points: &[i64],
) -> Vec<HashMap<String, Decimal>> {
    let mut legs: Vec<&(i64, String, Decimal)> = legs.iter().collect();
    legs.sort_by_key(|l| std::cmp::Reverse(l.0));
    let mut running = now.clone();
    let mut next_leg = 0;
    let mut out = vec![HashMap::new(); points.len()];
    for (i, &point) in points.iter().enumerate().rev() {
        // Undo everything that happened after this point.
        while let Some((ts, asset, amount)) = legs.get(next_leg).copied() {
            if *ts <= point {
                break;
            }
            *running.entry(asset.clone()).or_default() -= *amount;
            next_leg += 1;
        }
        out[i] = running
            .iter()
            .filter(|(_, a)| a.is_sign_positive() && !a.is_zero())
            .map(|(k, v)| (k.clone(), *v))
            .collect();
    }
    out
}

/// Price of a sorted `(timestamp, price)` series at `ts`: the latest sample at
/// or before it, else the first one after.
pub fn price_at(series: &[(i64, Decimal)], ts: i64) -> Option<Decimal> {
    if series.is_empty() {
        return None;
    }
    let idx = series.partition_point(|(t, _)| *t <= ts);
    if idx == 0 {
        series.first().map(|(_, p)| *p)
    } else {
        series.get(idx - 1).map(|(_, p)| *p)
    }
}

/// Value of each snapshot. `price_key` maps asset ids to price series keys;
/// the last point uses `current_prices` so it matches the dashboard total.
pub fn value_series(
    points: &[i64],
    holdings: &[HashMap<String, Decimal>],
    price_key: &HashMap<String, String>,
    series: &HashMap<String, Vec<(i64, Decimal)>>,
    current_prices: &HashMap<String, Decimal>,
) -> Vec<Decimal> {
    points
        .iter()
        .zip(holdings)
        .enumerate()
        .map(|(i, (&ts, snapshot))| {
            let last = i + 1 == points.len();
            snapshot
                .iter()
                .filter_map(|(asset, amount)| {
                    let key = price_key.get(asset)?;
                    let price = if last {
                        current_prices.get(key).copied()
                    } else {
                        series.get(key).and_then(|s| price_at(s, ts))
                    }
                    .or_else(|| current_prices.get(key).copied())?;
                    Some(*amount * price)
                })
                .sum()
        })
        .collect()
}

/// Sample spacing for a range: hourly up to two days, 4-hourly up to two
/// weeks, daily beyond.
pub fn step_for(span_ms: i64) -> i64 {
    const HOUR: i64 = 3_600_000;
    match span_ms {
        s if s <= 2 * 24 * HOUR => HOUR,
        s if s <= 14 * 24 * HOUR => 4 * HOUR,
        _ => 24 * HOUR,
    }
}

/// Evenly spaced sample times from `from` to `to`, always ending at `to`.
pub fn sample_points(from: i64, to: i64, step: i64) -> Vec<i64> {
    let mut points = Vec::new();
    let mut t = from;
    while t < to {
        points.push(t);
        t += step;
    }
    points.push(to);
    points
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(n: i64) -> Decimal {
        Decimal::from(n)
    }

    #[test]
    fn replays_balances_backwards() {
        let now = HashMap::from([("eth".to_string(), d(5))]);
        // Received 3 at t=20, sent 1 at t=40.
        let legs = vec![
            (20, "eth".to_string(), d(3)),
            (40, "eth".to_string(), d(-1)),
        ];
        let snaps = holdings_at(&now, &legs, &[10, 30, 50]);
        assert_eq!(snaps[0]["eth"], d(3));
        assert_eq!(snaps[1]["eth"], d(6));
        assert_eq!(snaps[2]["eth"], d(5));
    }

    #[test]
    fn assets_appear_only_while_held() {
        let now = HashMap::new();
        let legs = vec![(20, "sol".to_string(), d(-2))];
        let snaps = holdings_at(&now, &legs, &[10, 30]);
        assert_eq!(snaps[0]["sol"], d(2));
        assert!(snaps[1].is_empty());
    }

    #[test]
    fn prices_use_latest_sample_before() {
        let s = vec![(10, d(1)), (20, d(2))];
        assert_eq!(price_at(&s, 5), Some(d(1)));
        assert_eq!(price_at(&s, 15), Some(d(1)));
        assert_eq!(price_at(&s, 25), Some(d(2)));
        assert_eq!(price_at(&[], 1), None);
    }

    #[test]
    fn values_snapshots_and_pins_last_to_current_price() {
        let holdings = vec![
            HashMap::from([("eth".to_string(), d(2))]),
            HashMap::from([("eth".to_string(), d(2))]),
        ];
        let keys = HashMap::from([("eth".to_string(), "coingecko:ethereum".to_string())]);
        let series = HashMap::from([("coingecko:ethereum".to_string(), vec![(0, d(100))])]);
        let current = HashMap::from([("coingecko:ethereum".to_string(), d(150))]);
        let values = value_series(&[10, 20], &holdings, &keys, &series, &current);
        assert_eq!(values, vec![d(200), d(300)]);
    }

    #[test]
    fn steps_and_points() {
        assert_eq!(step_for(3_600_000 * 24), 3_600_000);
        assert_eq!(step_for(3_600_000 * 24 * 7), 3_600_000 * 4);
        assert_eq!(step_for(3_600_000 * 24 * 30), 3_600_000 * 24);
        assert_eq!(sample_points(0, 25, 10), vec![0, 10, 20, 25]);
    }
}
