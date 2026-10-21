//! Reads stored history in the shape the PnL engine needs.

use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
};

use rusqlite::Connection;
use rust_decimal::Decimal;

use super::history::{TxAsset, current_holdings};
use crate::{chains::Chain, engine::pnl::Entry, error::Result, history::classify::Kind};

/// Price keys with a current price. Only these assets take part in PnL:
/// unpriced tokens are almost always spam.
fn priced_keys(conn: &Connection) -> Result<HashSet<String>> {
    let mut stmt = conn.prepare("SELECT price_key FROM prices")?;
    let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

/// Every stored wallet entry, oldest first, with amounts grouped by price key.
pub fn entries(conn: &Connection) -> Result<Vec<Entry>> {
    let priced = priced_keys(conn)?;
    let mut stmt = conn.prepare(
        "SELECT e.tx_id, e.wallet_id, t.timestamp, COALESCE(e.kind_override, e.kind),
                e.kind_override IS NOT NULL, e.fee, e.fee_asset_id, fa.price_key,
                l.asset_id, l.amount, a.price_key
           FROM tx_entries e
           JOIN transactions t ON t.id = e.tx_id
           LEFT JOIN tx_legs l ON l.tx_id = e.tx_id AND l.wallet_id = e.wallet_id
           LEFT JOIN assets a ON a.id = l.asset_id
           LEFT JOIN assets fa ON fa.id = e.fee_asset_id
          ORDER BY t.timestamp, e.tx_id, e.wallet_id",
    )?;
    let mut rows = stmt.query([])?;
    let mut out: Vec<Entry> = Vec::new();
    let mut current: Option<(String, i64)> = None;
    while let Some(r) = rows.next()? {
        let tx_id: String = r.get(0)?;
        let wallet_id: i64 = r.get(1)?;
        let fee = r
            .get::<_, Option<String>>(5)?
            .and_then(|f| Decimal::from_str(&f).ok());
        let fee_asset: Option<String> = r.get(6)?;
        let fee_key: Option<String> = r.get(7)?;

        let key = (tx_id.clone(), wallet_id);
        if current.as_ref() != Some(&key) {
            let kind = Kind::parse(&r.get::<_, String>(3)?).unwrap_or(Kind::Contract);
            out.push(Entry {
                tx_id,
                t: r.get(2)?,
                kind,
                overridden: r.get(4)?,
                deltas: Vec::new(),
                fee: fee
                    .zip(fee_key.clone())
                    .filter(|(_, k)| priced.contains(k))
                    .map(|(f, k)| (k, f)),
            });
            current = Some(key);
        }
        let (Some(asset), Some(amount), Some(pool)) = (
            r.get::<_, Option<String>>(8)?,
            r.get::<_, Option<String>>(9)?
                .and_then(|a| Decimal::from_str(&a).ok()),
            r.get::<_, Option<String>>(10)?,
        ) else {
            continue;
        };
        if !priced.contains(&pool) {
            continue;
        }
        // Legs include the fee; the engine wants the transfer alone.
        let transfer = match (&fee_asset, fee) {
            (Some(fa), Some(f)) if *fa == asset => amount + f,
            _ => amount,
        };
        if transfer.is_zero() {
            continue;
        }
        let entry = out.last_mut().expect("entry pushed above");
        match entry.deltas.iter_mut().find(|(p, _)| *p == pool) {
            Some((_, a)) => *a += transfer,
            None => entry.deltas.push((pool, transfer)),
        }
    }
    Ok(out)
}

/// Current amount and price per price key (summed across chains and wallets).
pub fn pool_holdings(conn: &Connection) -> Result<HashMap<String, (Decimal, Decimal)>> {
    let mut out: HashMap<String, (Decimal, Decimal)> = HashMap::new();
    for (_, key, amount, price) in current_holdings(conn, None)? {
        let entry = out.entry(key).or_insert((Decimal::ZERO, price));
        entry.0 += amount;
    }
    Ok(out)
}

/// Current price of every priced key, held or not.
pub fn current_prices(conn: &Connection) -> Result<HashMap<String, Decimal>> {
    let mut stmt = conn.prepare("SELECT price_key, price_usd FROM prices")?;
    let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
    Ok(rows
        .filter_map(|r| r.ok())
        .filter_map(|(k, p)| Decimal::from_str(&p).ok().map(|p| (k, p)))
        .collect())
}

/// One representative asset per price key, for names and logos: a native coin
/// or a held one first.
pub fn pool_assets(conn: &Connection) -> Result<HashMap<String, TxAsset>> {
    let mut stmt = conn.prepare(
        "SELECT a.price_key, a.id, a.chain, a.symbol, a.name, a.logo_url
           FROM assets a
          ORDER BY a.contract IS NOT NULL,
                   NOT EXISTS (SELECT 1 FROM balances b WHERE b.asset_id = a.id),
                   a.logo_url IS NULL",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, String>(3)?,
            r.get::<_, String>(4)?,
            r.get::<_, Option<String>>(5)?,
        ))
    })?;
    let mut out = HashMap::new();
    for (key, id, chain, symbol, name, logo_url) in rows.filter_map(|r| r.ok()) {
        let Ok(chain) = Chain::from_str(&chain) else {
            continue;
        };
        out.entry(key).or_insert(TxAsset {
            id,
            chain,
            symbol,
            name,
            logo_url,
        });
    }
    Ok(out)
}

/// Daily price history of `keys`, as Unix ms series.
pub fn price_series(
    conn: &Connection,
    keys: &[String],
) -> Result<HashMap<String, Vec<(i64, Decimal)>>> {
    let all = super::history::daily_prices(conn, keys, i64::MIN / 2, i64::MAX / 2)?;
    Ok(all
        .into_iter()
        .map(|(k, s)| (k, s.into_iter().map(|(d, p)| (d * 86_400_000, p)).collect()))
        .collect())
}

/// Whether every wallet's history has been fully backfilled, and the oldest
/// synced transaction.
pub fn coverage(conn: &Connection) -> Result<(bool, Option<i64>)> {
    let pending: i64 = conn.query_row(
        "SELECT COUNT(*) FROM history_cursors WHERE complete = 0",
        [],
        |r| r.get(0),
    )?;
    let oldest: Option<i64> =
        conn.query_row("SELECT MIN(timestamp) FROM transactions", [], |r| r.get(0))?;
    Ok((pending == 0, oldest))
}

#[cfg(test)]
mod tests {
    use rusqlite::params;

    use super::*;
    use crate::{
        chains::AssetMeta,
        chains::Family,
        db::{
            history::{insert_txs, reclassify, update_entry},
            open_in_memory,
            repo::{self, NewWallet, WalletKind},
        },
        history::RawTx,
    };

    #[test]
    fn entries_drop_fees_from_transfers_and_skip_unpriced() {
        let mut conn = open_in_memory().unwrap();
        let w = repo::insert_wallet(
            &mut conn,
            &NewWallet {
                label: "a",
                family: Family::Evm,
                kind: WalletKind::Address,
                address: "0xa",
                input: "0xa",
            },
        )
        .unwrap()
        .id;
        let eth = AssetMeta::native(Chain::Ethereum, "ETH", "Ether", 18, "ethereum");
        let junk = AssetMeta::token(Chain::Ethereum, "0xjunk", 18);
        let mut tx = RawTx::new("0x1", 1_000);
        tx.add(eth.clone(), Decimal::NEGATIVE_ONE, Some("0xb".into()));
        tx.add(junk, Decimal::from(5), None);
        tx.fee = Some((eth, Decimal::new(1, 2)));
        let ids = insert_txs(&mut conn, w, Chain::Ethereum, &[tx]).unwrap();
        reclassify(&mut conn, &ids).unwrap();
        update_entry(&conn, &ids[0], w, Some("sale"), None).unwrap();
        conn.execute(
            "INSERT INTO prices (price_key, price_usd, updated_at) VALUES ('coingecko:ethereum', '2000', 0)",
            params![],
        )
        .unwrap();

        let entries = entries(&conn).unwrap();
        assert_eq!(entries.len(), 1);
        let e = &entries[0];
        assert_eq!(e.kind, Kind::Sale);
        assert!(e.overridden);
        assert_eq!(
            e.deltas,
            vec![("coingecko:ethereum".to_string(), Decimal::NEGATIVE_ONE)]
        );
        assert_eq!(
            e.fee,
            Some(("coingecko:ethereum".to_string(), Decimal::new(1, 2)))
        );
    }
}
