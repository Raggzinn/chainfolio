//! Storage for transaction history, sync cursors, daily prices and settings.

use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
};

use rusqlite::{Connection, OptionalExtension, params, params_from_iter};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use super::repo::{decimal_column, upsert_asset};
use crate::{
    chains::{Chain, Family},
    error::{Error, Result},
    history::{
        RawTx,
        classify::{EntryFacts, Kind, classify},
    },
};

// ------------------------------------------------------------------ settings

pub fn get_setting(conn: &Connection, key: &str) -> Result<Option<String>> {
    Ok(conn
        .query_row("SELECT value FROM settings WHERE key = ?1", [key], |r| {
            r.get(0)
        })
        .optional()?)
}

pub fn set_setting(conn: &Connection, key: &str, value: Option<&str>) -> Result<()> {
    match value.map(str::trim).filter(|v| !v.is_empty()) {
        Some(value) => conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT (key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?,
        None => conn.execute("DELETE FROM settings WHERE key = ?1", [key])?,
    };
    Ok(())
}

// ------------------------------------------------------------------- cursors

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Cursor {
    pub newest: Option<String>,
    pub backfill: Option<String>,
    pub complete: bool,
}

pub fn get_cursor(conn: &Connection, wallet_id: i64, chain: Chain) -> Result<Option<Cursor>> {
    Ok(conn
        .query_row(
            "SELECT newest, backfill, complete FROM history_cursors WHERE wallet_id = ?1 AND chain = ?2",
            params![wallet_id, chain.as_str()],
            |r| {
                Ok(Cursor {
                    newest: r.get(0)?,
                    backfill: r.get(1)?,
                    complete: r.get::<_, i64>(2)? != 0,
                })
            },
        )
        .optional()?)
}

pub fn set_cursor(conn: &Connection, wallet_id: i64, chain: Chain, cursor: &Cursor) -> Result<()> {
    conn.execute(
        "INSERT INTO history_cursors (wallet_id, chain, newest, backfill, complete, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT (wallet_id, chain) DO UPDATE SET newest = excluded.newest,
           backfill = excluded.backfill, complete = excluded.complete, updated_at = excluded.updated_at",
        params![
            wallet_id,
            chain.as_str(),
            cursor.newest,
            cursor.backfill,
            cursor.complete as i64,
            super::now_ms()
        ],
    )?;
    Ok(())
}

/// Addresses whose history belongs to `wallet`: the derived addresses of an
/// extended key, or the single address otherwise.
pub fn owner_addresses(conn: &Connection, wallet_id: i64, address: &str) -> Result<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT address FROM wallet_addresses WHERE wallet_id = ?1 AND address <> ?2 ORDER BY id",
    )?;
    let derived: Vec<String> = stmt
        .query_map(params![wallet_id, address], |r| r.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    Ok(if derived.is_empty() {
        vec![address.to_string()]
    } else {
        derived
    })
}

/// Hashes on `chain` already stored for `wallet_id`.
pub fn known_hashes(conn: &Connection, wallet_id: i64, chain: Chain) -> Result<HashSet<String>> {
    let mut stmt = conn.prepare(
        "SELECT t.hash FROM tx_entries e JOIN transactions t ON t.id = e.tx_id
         WHERE e.wallet_id = ?1 AND t.chain = ?2",
    )?;
    let rows = stmt.query_map(params![wallet_id, chain.as_str()], |r| r.get(0))?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

fn tx_id(chain: Chain, hash: &str) -> String {
    format!("{chain}:{hash}")
}

/// Stores transactions new to `wallet_id` and returns their ids. Already
/// stored ones are left untouched so user edits survive re-syncs.
pub fn insert_txs(
    conn: &mut Connection,
    wallet_id: i64,
    chain: Chain,
    txs: &[RawTx],
) -> Result<Vec<String>> {
    let known = known_hashes(conn, wallet_id, chain)?;
    let tx = conn.transaction()?;
    let mut inserted = Vec::new();
    for raw in txs.iter().filter(|t| !known.contains(&t.hash)) {
        let id = tx_id(chain, &raw.hash);
        tx.execute(
            "INSERT INTO transactions (id, chain, hash, timestamp, failed) VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT (id) DO UPDATE SET timestamp = MAX(timestamp, excluded.timestamp),
               failed = MAX(failed, excluded.failed)",
            params![id, chain.as_str(), raw.hash, raw.timestamp, raw.failed as i64],
        )?;
        let (fee, fee_asset) = match &raw.fee {
            Some((asset, amount)) => {
                upsert_asset(&tx, asset, None)?;
                (Some(amount.to_string()), Some(asset.id()))
            }
            None => (None, None),
        };
        tx.execute(
            "INSERT INTO tx_entries (tx_id, wallet_id, kind, fee, fee_asset_id) VALUES (?1, ?2, 'receive', ?3, ?4)",
            params![id, wallet_id, fee, fee_asset],
        )?;
        // Legs hold the net balance change, so the fee is folded in.
        let mut legs: Vec<(String, Decimal, Option<String>)> = Vec::new();
        for delta in &raw.deltas {
            upsert_asset(&tx, &delta.asset, None)?;
            legs.push((delta.asset.id(), delta.amount, delta.counterparty.clone()));
        }
        if let Some((asset, amount)) = &raw.fee {
            match legs.iter_mut().find(|(a, _, _)| *a == asset.id()) {
                Some(leg) => leg.1 -= *amount,
                None => legs.push((asset.id(), -*amount, None)),
            }
        }
        for (asset_id, amount, counterparty) in legs.into_iter().filter(|(_, a, _)| !a.is_zero()) {
            tx.execute(
                "INSERT INTO tx_legs (tx_id, wallet_id, asset_id, amount, counterparty) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![id, wallet_id, asset_id, amount.to_string(), counterparty],
            )?;
        }
        inserted.push(id);
    }
    tx.commit()?;
    Ok(inserted)
}

/// Every tracked address, lowercased, for internal-transfer detection.
fn own_addresses(conn: &Connection) -> Result<HashSet<String>> {
    let mut stmt =
        conn.prepare("SELECT address FROM wallets UNION SELECT address FROM wallet_addresses")?;
    let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
    Ok(rows
        .filter_map(|r| r.ok())
        .map(|a| a.to_lowercase())
        .collect())
}

/// Recomputes the automatic category of every wallet entry of `tx_ids`.
pub fn reclassify(conn: &mut Connection, tx_ids: &[String]) -> Result<()> {
    if tx_ids.is_empty() {
        return Ok(());
    }
    let own = own_addresses(conn)?;
    let tx = conn.transaction()?;
    for id in tx_ids {
        let failed: bool = tx
            .query_row("SELECT failed FROM transactions WHERE id = ?1", [id], |r| {
                r.get::<_, i64>(0)
            })
            .optional()?
            .is_some_and(|f| f != 0);
        let mut entries: HashMap<i64, EntryFacts> = HashMap::new();
        {
            let mut stmt = tx.prepare("SELECT wallet_id FROM tx_entries WHERE tx_id = ?1")?;
            let rows = stmt.query_map([id], |r| r.get::<_, i64>(0))?;
            for row in rows {
                let wallet = row?;
                entries.insert(
                    wallet,
                    EntryFacts {
                        failed,
                        ..EntryFacts::default()
                    },
                );
            }
            let mut stmt = tx.prepare(
                "SELECT l.wallet_id, l.asset_id, l.amount, l.counterparty, e.fee, e.fee_asset_id
                 FROM tx_legs l JOIN tx_entries e ON e.tx_id = l.tx_id AND e.wallet_id = l.wallet_id
                 WHERE l.tx_id = ?1",
            )?;
            let rows = stmt.query_map([id], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, Option<String>>(3)?,
                    r.get::<_, Option<String>>(4)?,
                    r.get::<_, Option<String>>(5)?,
                ))
            })?;
            for row in rows {
                let (wallet, asset, amount, counterparty, fee, fee_asset) = row?;
                let Ok(mut amount) = Decimal::from_str(&amount) else {
                    continue;
                };
                // Classification looks at transfers, so take the fee back out.
                if fee_asset.as_deref() == Some(asset.as_str()) {
                    amount += decimal_column(fee).unwrap_or_default();
                }
                let facts = entries.entry(wallet).or_default();
                if !amount.is_zero() {
                    facts.deltas.push((asset, amount));
                }
                facts.counterparties.extend(counterparty);
            }
        }
        for (wallet, kind) in classify(&entries, &own) {
            tx.execute(
                "UPDATE tx_entries SET kind = ?1 WHERE tx_id = ?2 AND wallet_id = ?3",
                params![kind.as_str(), id, wallet],
            )?;
        }
    }
    tx.commit()?;
    Ok(())
}

pub fn update_entry(
    conn: &Connection,
    tx_id: &str,
    wallet_id: i64,
    kind: Option<&str>,
    note: Option<&str>,
) -> Result<()> {
    let kind = match kind {
        Some(k) => {
            Some(Kind::parse(k).ok_or_else(|| Error::provider(format!("unknown kind {k}")))?)
        }
        None => None,
    };
    let note = note.map(str::trim).filter(|n| !n.is_empty());
    let changed = conn.execute(
        "UPDATE tx_entries SET kind_override = ?1, note = ?2 WHERE tx_id = ?3 AND wallet_id = ?4",
        params![kind.map(Kind::as_str), note, tx_id, wallet_id],
    )?;
    if changed == 0 {
        return Err(Error::NotFound);
    }
    Ok(())
}

// --------------------------------------------------------------------- query

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TxQuery {
    pub wallet_id: Option<i64>,
    pub chain: Option<Chain>,
    pub asset_id: Option<String>,
    pub kind: Option<Kind>,
    /// Unix ms, inclusive.
    pub from: Option<i64>,
    /// Unix ms, inclusive.
    pub to: Option<i64>,
    /// Matches hash, asset symbol, wallet label, counterparty or note.
    pub search: Option<String>,
    #[serde(default)]
    pub include_spam: bool,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TxAsset {
    pub id: String,
    pub chain: Chain,
    pub symbol: String,
    pub name: String,
    pub logo_url: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TxLeg {
    pub asset: TxAsset,
    /// Transfer amount (positive in, negative out), fee excluded.
    pub amount: Decimal,
    /// USD value at the transaction date when a daily price is known.
    pub value_usd: Option<Decimal>,
    pub counterparty: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TxFee {
    pub asset: TxAsset,
    pub amount: Decimal,
    pub value_usd: Option<Decimal>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TxItem {
    pub id: String,
    pub chain: Chain,
    pub hash: String,
    pub timestamp: i64,
    pub wallet_id: i64,
    pub wallet_label: String,
    pub family: Family,
    pub kind: Kind,
    pub auto_kind: Kind,
    pub note: Option<String>,
    pub failed: bool,
    pub legs: Vec<TxLeg>,
    pub fee: Option<TxFee>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TxPage {
    pub items: Vec<TxItem>,
    pub total: i64,
}

fn asset_row(conn: &Connection, id: &str) -> Result<Option<(TxAsset, String)>> {
    Ok(conn
        .query_row(
            "SELECT chain, symbol, name, logo_url, price_key FROM assets WHERE id = ?1",
            [id],
            |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, Option<String>>(3)?,
                    r.get::<_, String>(4)?,
                ))
            },
        )
        .optional()?
        .and_then(|(chain, symbol, name, logo_url, price_key)| {
            Some((
                TxAsset {
                    id: id.to_string(),
                    chain: Chain::from_str(&chain).ok()?,
                    symbol,
                    name,
                    logo_url,
                },
                price_key,
            ))
        }))
}

/// Daily close for `price_key` on the day of `timestamp` (ms), if cached.
fn price_on(conn: &Connection, price_key: &str, timestamp: i64) -> Result<Option<Decimal>> {
    let day = timestamp.div_euclid(86_400_000);
    let price: Option<String> = conn
        .query_row(
            "SELECT price_usd FROM price_history WHERE price_key = ?1 AND day = ?2",
            params![price_key, day],
            |r| r.get(0),
        )
        .optional()?;
    Ok(decimal_column(price))
}

pub fn list_transactions(conn: &Connection, query: &TxQuery) -> Result<TxPage> {
    let mut filters = vec!["1 = 1".to_string()];
    let mut args: Vec<rusqlite::types::Value> = Vec::new();
    // Adds a filter whose single `{}` placeholder is the next argument.
    fn bind(
        filters: &mut Vec<String>,
        args: &mut Vec<rusqlite::types::Value>,
        sql: &str,
        value: rusqlite::types::Value,
    ) {
        args.push(value);
        filters.push(sql.replace("{}", &format!("?{}", args.len())));
    }
    if let Some(wallet) = query.wallet_id {
        bind(&mut filters, &mut args, "e.wallet_id = {}", wallet.into());
    }
    if let Some(chain) = query.chain {
        bind(
            &mut filters,
            &mut args,
            "t.chain = {}",
            chain.as_str().to_string().into(),
        );
    }
    if let Some(asset) = &query.asset_id {
        bind(
            &mut filters,
            &mut args,
            "EXISTS (SELECT 1 FROM tx_legs l WHERE l.tx_id = e.tx_id AND l.wallet_id = e.wallet_id AND l.asset_id = {})",
            asset.clone().into(),
        );
    }
    if let Some(kind) = query.kind {
        bind(
            &mut filters,
            &mut args,
            "COALESCE(e.kind_override, e.kind) = {}",
            kind.as_str().to_string().into(),
        );
    } else if !query.include_spam {
        filters.push("COALESCE(e.kind_override, e.kind) <> 'spam'".into());
    }
    if let Some(from) = query.from {
        bind(&mut filters, &mut args, "t.timestamp >= {}", from.into());
    }
    if let Some(to) = query.to {
        bind(&mut filters, &mut args, "t.timestamp <= {}", to.into());
    }
    if let Some(search) = query
        .search
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        // `!` escapes LIKE wildcards typed by the user.
        let like = format!(
            "%{}%",
            search
                .replace('!', "!!")
                .replace('%', "!%")
                .replace('_', "!_")
        );
        bind(
            &mut filters,
            &mut args,
            "(t.hash LIKE {} ESCAPE '!' OR w.label LIKE {} ESCAPE '!' OR e.note LIKE {} ESCAPE '!'
              OR EXISTS (SELECT 1 FROM tx_legs l JOIN assets a ON a.id = l.asset_id
                         WHERE l.tx_id = e.tx_id AND l.wallet_id = e.wallet_id
                           AND (a.symbol LIKE {} ESCAPE '!' OR a.name LIKE {} ESCAPE '!'
                                OR l.counterparty LIKE {} ESCAPE '!')))",
            like.into(),
        );
    }
    let where_sql = filters.join(" AND ");
    let from_sql = "FROM tx_entries e JOIN transactions t ON t.id = e.tx_id JOIN wallets w ON w.id = e.wallet_id";

    let total: i64 = conn.query_row(
        &format!("SELECT COUNT(*) {from_sql} WHERE {where_sql}"),
        params_from_iter(args.iter()),
        |r| r.get(0),
    )?;

    let limit = query.limit.unwrap_or(50).clamp(1, 500);
    let offset = query.offset.unwrap_or(0).max(0);
    let mut stmt = conn.prepare(&format!(
        "SELECT e.tx_id, t.chain, t.hash, t.timestamp, e.wallet_id, w.label, w.family, e.kind,
                e.kind_override, e.note, t.failed, e.fee, e.fee_asset_id
         {from_sql} WHERE {where_sql}
         ORDER BY t.timestamp DESC, e.tx_id LIMIT {limit} OFFSET {offset}"
    ))?;
    let rows = stmt.query_map(params_from_iter(args.iter()), |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, i64>(3)?,
            r.get::<_, i64>(4)?,
            r.get::<_, String>(5)?,
            r.get::<_, String>(6)?,
            r.get::<_, String>(7)?,
            r.get::<_, Option<String>>(8)?,
            r.get::<_, Option<String>>(9)?,
            r.get::<_, i64>(10)?,
            r.get::<_, Option<String>>(11)?,
            r.get::<_, Option<String>>(12)?,
        ))
    })?;

    let mut items = Vec::new();
    for row in rows {
        let (
            id,
            chain,
            hash,
            timestamp,
            wallet_id,
            wallet_label,
            family,
            kind,
            kind_override,
            note,
            failed,
            fee,
            fee_asset,
        ) = row?;
        let (Ok(chain), Ok(family)) = (Chain::from_str(&chain), Family::from_str(&family)) else {
            continue;
        };
        let auto_kind = Kind::parse(&kind).unwrap_or(Kind::Contract);
        let kind = kind_override
            .as_deref()
            .and_then(Kind::parse)
            .unwrap_or(auto_kind);
        let fee_amount = decimal_column(fee);

        let fee = match (fee_amount, fee_asset.as_deref()) {
            (Some(amount), Some(asset_id)) => asset_row(conn, asset_id)?.map(|(asset, key)| {
                let value_usd = price_on(conn, &key, timestamp)
                    .ok()
                    .flatten()
                    .map(|p| p * amount);
                TxFee {
                    asset,
                    amount,
                    value_usd,
                }
            }),
            _ => None,
        };

        let mut legs = Vec::new();
        let mut leg_stmt = conn.prepare_cached(
            "SELECT asset_id, amount, counterparty FROM tx_legs WHERE tx_id = ?1 AND wallet_id = ?2",
        )?;
        let leg_rows = leg_stmt.query_map(params![id, wallet_id], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Option<String>>(2)?,
            ))
        })?;
        for leg in leg_rows {
            let (asset_id, amount, counterparty) = leg?;
            let Ok(mut amount) = Decimal::from_str(&amount) else {
                continue;
            };
            if let (Some(f), Some(fa)) = (&fee, fee_asset.as_deref()) {
                if fa == asset_id {
                    amount += f.amount;
                }
            }
            if amount.is_zero() {
                continue;
            }
            let Some((asset, key)) = asset_row(conn, &asset_id)? else {
                continue;
            };
            let value_usd = price_on(conn, &key, timestamp)?.map(|p| p * amount);
            legs.push(TxLeg {
                asset,
                amount,
                value_usd,
                counterparty,
            });
        }
        legs.sort_by_key(|l| l.amount);

        items.push(TxItem {
            id,
            chain,
            hash,
            timestamp,
            wallet_id,
            wallet_label,
            family,
            kind,
            auto_kind,
            note,
            failed: failed != 0,
            legs,
            fee,
        });
    }
    Ok(TxPage { items, total })
}

/// CSV of the transactions matching `query` (all pages), one row per leg.
pub fn transactions_csv(conn: &Connection, query: &TxQuery) -> Result<String> {
    let mut query = query.clone();
    query.limit = Some(500);
    query.offset = Some(0);
    let mut out = String::from(
        "date_utc,wallet,chain,type,asset,amount,value_usd,fee,fee_asset,hash,counterparty,note\n",
    );
    loop {
        let page = list_transactions(conn, &query)?;
        let count = page.items.len() as i64;
        for item in &page.items {
            let date = utc_iso(item.timestamp);
            let fee = item
                .fee
                .as_ref()
                .map(|f| f.amount.to_string())
                .unwrap_or_default();
            let fee_asset = item
                .fee
                .as_ref()
                .map(|f| f.asset.symbol.clone())
                .unwrap_or_default();
            let legs: Vec<Option<&TxLeg>> = if item.legs.is_empty() {
                vec![None]
            } else {
                item.legs.iter().map(Some).collect()
            };
            for leg in legs {
                let row = [
                    date.clone(),
                    item.wallet_label.clone(),
                    item.chain.as_str().into(),
                    item.kind.as_str().into(),
                    leg.map(|l| l.asset.symbol.clone()).unwrap_or_default(),
                    leg.map(|l| l.amount.to_string()).unwrap_or_default(),
                    leg.and_then(|l| l.value_usd)
                        .map(|v| v.round_dp(2).to_string())
                        .unwrap_or_default(),
                    fee.clone(),
                    fee_asset.clone(),
                    item.hash.clone(),
                    leg.and_then(|l| l.counterparty.clone()).unwrap_or_default(),
                    item.note.clone().unwrap_or_default(),
                ];
                out.push_str(
                    &row.iter()
                        .map(|f| csv_field(f))
                        .collect::<Vec<_>>()
                        .join(","),
                );
                out.push('\n');
            }
        }
        if count < 500 {
            break;
        }
        query.offset = Some(query.offset.unwrap_or(0) + count);
    }
    Ok(out)
}

fn csv_field(value: &str) -> String {
    if value.contains([',', '"', '\n']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

/// `YYYY-MM-DDTHH:MM:SSZ` for a Unix ms timestamp.
pub fn utc_iso(ms: i64) -> String {
    let secs = ms.div_euclid(1000);
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    // Civil from days (Howard Hinnant).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60
    )
}

// ------------------------------------------------------------- price history

pub fn store_daily_prices(
    conn: &mut Connection,
    prices: &HashMap<String, Vec<(i64, Decimal)>>,
) -> Result<()> {
    let tx = conn.transaction()?;
    for (key, series) in prices {
        for (day, price) in series {
            tx.execute(
                "INSERT INTO price_history (price_key, day, price_usd) VALUES (?1, ?2, ?3)
                 ON CONFLICT (price_key, day) DO UPDATE SET price_usd = excluded.price_usd",
                params![key, day, price.to_string()],
            )?;
        }
    }
    tx.commit()?;
    Ok(())
}

/// Cached daily closes for `keys` between `from_day` and `to_day` inclusive.
pub fn daily_prices(
    conn: &Connection,
    keys: &[String],
    from_day: i64,
    to_day: i64,
) -> Result<HashMap<String, Vec<(i64, Decimal)>>> {
    let mut out: HashMap<String, Vec<(i64, Decimal)>> = HashMap::new();
    let mut stmt = conn.prepare_cached(
        "SELECT day, price_usd FROM price_history WHERE price_key = ?1 AND day BETWEEN ?2 AND ?3 ORDER BY day",
    )?;
    for key in keys {
        let rows = stmt.query_map(params![key, from_day, to_day], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
        })?;
        let series: Vec<(i64, Decimal)> = rows
            .filter_map(|r| r.ok())
            .filter_map(|(day, price)| Decimal::from_str(&price).ok().map(|p| (day, p)))
            .collect();
        out.insert(key.clone(), series);
    }
    Ok(out)
}

/// Price keys of every asset that appears in a transaction of the last
/// `days`, plus held ones, with the first day each needs a price for.
pub fn assets_needing_prices(conn: &Connection, since_ms: i64) -> Result<Vec<(String, i64)>> {
    let mut stmt = conn.prepare(
        "SELECT a.price_key, MIN(t.timestamp) FROM tx_legs l
           JOIN transactions t ON t.id = l.tx_id
           JOIN assets a ON a.id = l.asset_id
           JOIN prices p ON p.price_key = a.price_key
         WHERE t.timestamp >= ?1
         GROUP BY a.price_key",
    )?;
    let rows = stmt.query_map([since_ms], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

/// Net leg movements per asset after `since_ms`, newest first:
/// `(timestamp, asset_id, amount)`.
pub fn legs_since(
    conn: &Connection,
    since_ms: i64,
    asset_ids: Option<&[String]>,
) -> Result<Vec<(i64, String, Decimal)>> {
    let mut stmt = conn.prepare(
        "SELECT t.timestamp, l.asset_id, l.amount FROM tx_legs l JOIN transactions t ON t.id = l.tx_id
         WHERE t.timestamp > ?1 AND t.failed = 0 ORDER BY t.timestamp DESC",
    )?;
    let rows = stmt.query_map([since_ms], |r| {
        Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
        ))
    })?;
    let filter: Option<HashSet<&String>> = asset_ids.map(|ids| ids.iter().collect());
    Ok(rows
        .filter_map(|r| r.ok())
        .filter(|(_, asset, _)| filter.as_ref().is_none_or(|f| f.contains(asset)))
        .filter_map(|(ts, asset, amount)| Decimal::from_str(&amount).ok().map(|a| (ts, asset, a)))
        .collect())
}

/// Current amount and price key of every held asset with a current price.
pub fn current_holdings(
    conn: &Connection,
    asset_ids: Option<&[String]>,
) -> Result<Vec<(String, String, Decimal, Decimal)>> {
    let mut stmt = conn.prepare(
        "SELECT b.asset_id, a.price_key, b.amount, p.price_usd FROM balances b
           JOIN assets a ON a.id = b.asset_id
           JOIN prices p ON p.price_key = a.price_key",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, String>(3)?,
        ))
    })?;
    let filter: Option<HashSet<&String>> = asset_ids.map(|ids| ids.iter().collect());
    let mut by_asset: HashMap<String, (String, Decimal, Decimal)> = HashMap::new();
    for row in rows.filter_map(|r| r.ok()) {
        let (asset, key, amount, price) = row;
        if filter.as_ref().is_some_and(|f| !f.contains(&asset)) {
            continue;
        }
        let (Ok(amount), Ok(price)) = (Decimal::from_str(&amount), Decimal::from_str(&price))
        else {
            continue;
        };
        let entry = by_asset.entry(asset).or_insert((key, Decimal::ZERO, price));
        entry.1 += amount;
    }
    Ok(by_asset
        .into_iter()
        .map(|(asset, (key, amount, price))| (asset, key, amount, price))
        .collect())
}

/// Price key of every asset that has ever had a balance or a leg.
pub fn price_keys_for(conn: &Connection, asset_ids: &[String]) -> Result<HashMap<String, String>> {
    let mut out = HashMap::new();
    let mut stmt = conn.prepare_cached("SELECT price_key FROM assets WHERE id = ?1")?;
    for id in asset_ids {
        if let Some(key) = stmt.query_row([id], |r| r.get::<_, String>(0)).optional()? {
            out.insert(id.clone(), key);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        chains::AssetMeta,
        db::{
            open_in_memory,
            repo::{self, NewWallet, WalletKind},
        },
        history::RawTx,
    };

    fn wallet(conn: &mut Connection, address: &str) -> i64 {
        repo::insert_wallet(
            conn,
            &NewWallet {
                label: address,
                family: Family::Evm,
                kind: WalletKind::Address,
                address,
                input: address,
            },
        )
        .unwrap()
        .id
    }

    fn eth() -> AssetMeta {
        AssetMeta::native(Chain::Ethereum, "ETH", "Ether", 18, "ethereum")
    }

    fn send(hash: &str, amount: i64, fee: Option<i64>) -> RawTx {
        let mut tx = RawTx::new(hash, 1_700_000_000_000);
        tx.add(eth(), Decimal::from(amount), Some("0xother".into()));
        tx.fee = fee.map(|f| (eth(), Decimal::new(f, 3)));
        tx
    }

    #[test]
    fn stores_once_and_folds_fees_into_legs() {
        let mut conn = open_in_memory().unwrap();
        let w = wallet(&mut conn, "0xa");
        let ids = insert_txs(&mut conn, w, Chain::Ethereum, &[send("0x1", -1, Some(2))]).unwrap();
        assert_eq!(ids.len(), 1);
        // Re-syncing the same transaction is a no-op.
        assert!(
            insert_txs(&mut conn, w, Chain::Ethereum, &[send("0x1", -1, Some(2))])
                .unwrap()
                .is_empty()
        );
        let leg: String = conn
            .query_row("SELECT amount FROM tx_legs", [], |r| r.get(0))
            .unwrap();
        assert_eq!(Decimal::from_str(&leg).unwrap(), Decimal::new(-1002, 3));

        reclassify(&mut conn, &ids).unwrap();
        let page = list_transactions(&conn, &TxQuery::default()).unwrap();
        assert_eq!(page.total, 1);
        let item = &page.items[0];
        assert_eq!(item.kind, Kind::Send);
        // The listed transfer excludes the fee again.
        assert_eq!(item.legs[0].amount, Decimal::NEGATIVE_ONE);
        assert_eq!(item.fee.as_ref().unwrap().amount, Decimal::new(2, 3));
    }

    #[test]
    fn transfer_between_wallets_becomes_internal() {
        let mut conn = open_in_memory().unwrap();
        let a = wallet(&mut conn, "0xa");
        let b = wallet(&mut conn, "0xb");
        let mut out = insert_txs(&mut conn, a, Chain::Ethereum, &[send("0x2", -1, None)]).unwrap();
        out.extend(insert_txs(&mut conn, b, Chain::Ethereum, &[send("0x2", 1, None)]).unwrap());
        out.dedup();
        reclassify(&mut conn, &out).unwrap();
        let page = list_transactions(&conn, &TxQuery::default()).unwrap();
        assert!(page.items.iter().all(|i| i.kind == Kind::Internal));
    }

    #[test]
    fn overrides_filters_and_csv() {
        let mut conn = open_in_memory().unwrap();
        let w = wallet(&mut conn, "0xa");
        let ids = insert_txs(&mut conn, w, Chain::Ethereum, &[send("0x3", 2, None)]).unwrap();
        reclassify(&mut conn, &ids).unwrap();
        update_entry(&conn, &ids[0], w, Some("income"), Some("salary, october")).unwrap();

        let income = list_transactions(
            &conn,
            &TxQuery {
                kind: Some(Kind::Income),
                ..TxQuery::default()
            },
        )
        .unwrap();
        assert_eq!(income.total, 1);
        assert_eq!(income.items[0].auto_kind, Kind::Receive);
        let search = list_transactions(
            &conn,
            &TxQuery {
                search: Some("salary".into()),
                ..TxQuery::default()
            },
        )
        .unwrap();
        assert_eq!(search.total, 1);
        let none = list_transactions(
            &conn,
            &TxQuery {
                search: Some("nope".into()),
                ..TxQuery::default()
            },
        )
        .unwrap();
        assert_eq!(none.total, 0);

        let csv = transactions_csv(&conn, &TxQuery::default()).unwrap();
        assert!(csv.contains("\"salary, october\""));
        assert!(csv.contains("2023-11-14T22:13:20Z"));
    }

    #[test]
    fn settings_round_trip() {
        let conn = open_in_memory().unwrap();
        set_setting(&conn, "k", Some(" v ")).unwrap();
        assert_eq!(get_setting(&conn, "k").unwrap().as_deref(), Some("v"));
        set_setting(&conn, "k", Some("")).unwrap();
        assert_eq!(get_setting(&conn, "k").unwrap(), None);
    }
}
