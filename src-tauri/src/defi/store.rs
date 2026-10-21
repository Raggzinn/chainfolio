//! Positions in SQLite, and how they adjust the portfolio: receipt tokens
//! and spam leave the holdings, position values join the totals.

use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
};

use rusqlite::{Connection, params};
use rust_decimal::Decimal;

use super::{DefiFetch, Position, lst, uniswap::fee_label, yields::Yields};
use crate::{chains::Chain, db::now_ms, error::Result};

const LST_SOURCE: &str = "lst";

/// Latest stored USD price per DefiLlama key.
fn prices(conn: &Connection) -> Result<HashMap<String, Decimal>> {
    let mut stmt = conn.prepare("SELECT price_key, price_usd FROM prices")?;
    let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
    let mut out = HashMap::new();
    for row in rows {
        let (key, price) = row?;
        if let Ok(price) = Decimal::from_str(&price) {
            out.insert(key, price);
        }
    }
    Ok(out)
}

/// Fills APYs that come from DefiLlama yields rather than the contracts.
fn apply_yields(position: &mut Position, yields: &Yields) {
    if position.protocol != "uniswap-v3" || position.apy.is_some() {
        return;
    }
    let pooled: Vec<&str> = position
        .tokens
        .iter()
        .filter(|t| t.role == super::TokenRole::Pooled)
        .filter_map(|t| t.contract.as_deref())
        .collect();
    if let ([t0, t1], Some(fee)) = (pooled.as_slice(), position.fee_tier) {
        position.apy = yields.uniswap(position.chain, t0, t1, &fee_label(fee));
    }
}

fn insert(
    tx: &rusqlite::Transaction<'_>,
    source: &str,
    position: &Position,
    now: i64,
) -> Result<()> {
    let data = serde_json::to_string(position)
        .map_err(|e| crate::error::Error::provider(e.to_string()))?;
    tx.execute(
        "INSERT INTO positions (wallet_id, source, chain, protocol, value_usd, in_balances, data, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            position.wallet_id,
            source,
            position.chain.as_str(),
            position.protocol,
            position.value_usd.map(|v| v.to_string()),
            position.in_balances,
            data,
            now
        ],
    )?;
    Ok(())
}

/// Positions worth less than this (USD) with no debt are leftovers of
/// closed positions, e.g. rounding wei in a Compound market.
const DUST_USD: Decimal = Decimal::from_parts(1, 0, 0, false, 2);

fn is_dust(position: &Position) -> bool {
    let has_debt = position
        .tokens
        .iter()
        .any(|t| t.role == super::TokenRole::Borrowed);
    let all_priced = position.tokens.iter().all(|t| t.value_usd.is_some());
    !has_debt && all_priced && position.value_usd.is_some_and(|v| v.abs() < DUST_USD)
}

/// Stores freshly fetched positions (valued with the prices just saved) and
/// rebuilds the liquid staking positions from the current balances.
pub fn save(conn: &mut Connection, fetch: DefiFetch, yields: Option<&Yields>) -> Result<()> {
    let prices = prices(conn)?;
    let now = now_ms();
    let tx = conn.transaction()?;
    for source in fetch.fetched {
        tx.execute(
            "DELETE FROM positions WHERE wallet_id = ?1 AND source = ?2",
            params![source.wallet_id, source.source],
        )?;
        for mut position in source.positions {
            if let Some(yields) = yields {
                apply_yields(&mut position, yields);
            }
            position.value_with(&prices);
            if is_dust(&position) {
                continue;
            }
            insert(&tx, &source.source, &position, now)?;
        }
    }

    // Keep the previous APYs when the yields feed is unavailable this time.
    let previous_apys: HashMap<(i64, String), Decimal> = if yields.is_none() {
        stored(&tx, Some(LST_SOURCE))?
            .into_iter()
            .filter_map(|p| Some(((p.wallet_id, p.tokens.first()?.price_key.clone()), p.apy?)))
            .collect()
    } else {
        HashMap::new()
    };
    tx.execute("DELETE FROM positions WHERE source = ?1", [LST_SOURCE])?;
    let held: Vec<(i64, String, String, String, String)> = {
        let mut stmt = tx.prepare(
            "SELECT b.wallet_id, a.id, a.symbol, a.price_key, b.amount
             FROM balances b JOIN assets a ON a.id = b.asset_id",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
        })?;
        rows.collect::<rusqlite::Result<_>>()?
    };
    for (wallet_id, asset_id, symbol, price_key, amount) in held {
        let (Some(lst), Ok(amount)) = (lst::find(&asset_id), Decimal::from_str(&amount)) else {
            continue;
        };
        let mut position = lst::position(lst, &symbol, &price_key, amount, yields);
        position.wallet_id = wallet_id;
        if position.tokens[0].apy.is_none() {
            position.tokens[0].apy = previous_apys.get(&(wallet_id, price_key)).copied();
        }
        position.value_with(&prices);
        insert(&tx, LST_SOURCE, &position, now)?;
    }
    tx.commit()?;
    Ok(())
}

fn stored(conn: &Connection, source: Option<&str>) -> Result<Vec<Position>> {
    let mut stmt = conn.prepare(
        "SELECT data FROM positions WHERE ?1 IS NULL OR source = ?1
         ORDER BY CAST(COALESCE(value_usd, '0') AS REAL) DESC, id",
    )?;
    let rows = stmt.query_map([source], |r| r.get::<_, String>(0))?;
    let mut out = Vec::new();
    for row in rows {
        // Rows written by an older layout are skipped rather than failing.
        if let Ok(position) = serde_json::from_str::<Position>(&row?) {
            out.push(position);
        }
    }
    Ok(out)
}

/// Every stored position, largest first.
pub fn list(conn: &Connection) -> Result<Vec<Position>> {
    stored(conn, None)
}

/// How positions and token flags change the plain balance totals.
#[derive(Debug, Default)]
pub struct Adjustments {
    /// (wallet, asset) balances that only represent a position.
    pub receipts: HashSet<(i64, String)>,
    /// Assets flagged as spam, hidden everywhere.
    pub spam: HashSet<String>,
    /// Position values not already part of the balances.
    pub extra: Vec<(i64, Chain, Decimal)>,
}

pub fn adjustments(conn: &Connection) -> Result<Adjustments> {
    let mut out = Adjustments::default();
    for position in list(conn)? {
        out.receipts.extend(
            position
                .receipts
                .iter()
                .map(|asset| (position.wallet_id, asset.clone())),
        );
        if let (false, Some(value)) = (position.in_balances, position.value_usd) {
            out.extra.push((position.wallet_id, position.chain, value));
        }
    }
    let mut stmt = conn.prepare("SELECT asset_id FROM token_flags WHERE spam = 1")?;
    let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
    for row in rows {
        out.spam.insert(row?);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        chains::{AssetMeta, Family, Holding},
        db::{
            open_in_memory,
            repo::{self, ChainBalances, NewWallet, SyncWrite, WalletKind},
        },
        defi::{PositionKind, PositionToken, SourcePositions, TokenRole},
        prices::Quote,
    };

    fn quote(price: i64) -> Quote {
        Quote {
            price_usd: Decimal::from(price),
            change_24h: None,
            symbol: None,
        }
    }

    #[test]
    fn positions_join_the_portfolio_without_double_counting() {
        let mut conn = open_in_memory().unwrap();
        let wallet = repo::insert_wallet(
            &mut conn,
            &NewWallet {
                label: "Main",
                family: Family::Evm,
                kind: WalletKind::Address,
                address: "0xa",
                input: "0xa",
            },
        )
        .unwrap();
        let steth = AssetMeta::token(
            Chain::Ethereum,
            "0xae7ab96520de3a18e5e111b5eaab095312d7fe84",
            18,
        )
        .with_names(Some("stETH".into()), None);
        let a_usdc = AssetMeta::token(Chain::Ethereum, "0xa0usdc", 6)
            .with_names(Some("aEthUSDC".into()), None);
        let spam = AssetMeta::token(Chain::Ethereum, "0xspam", 18);
        let mut write = SyncWrite::default();
        write.balances.push(ChainBalances {
            wallet_id: wallet.id,
            chain: Chain::Ethereum,
            holdings: [
                (steth.clone(), 2),
                (a_usdc.clone(), 1000),
                (spam.clone(), 5),
            ]
            .into_iter()
            .map(|(asset, amount)| Holding {
                asset,
                amount: Decimal::from(amount),
                locked: None,
            })
            .collect(),
        });
        write.quotes.insert(steth.price_key.clone(), quote(3000));
        write.quotes.insert(a_usdc.price_key.clone(), quote(1));
        write.quotes.insert(spam.price_key.clone(), quote(10));
        write.quotes.insert("ethereum:0xusdc".into(), quote(1));
        repo::save_sync(&mut conn, &write).unwrap();
        // 6000 stETH + 1000 aUSDC + 50 spam
        assert_eq!(
            repo::portfolio(&conn).unwrap().total_usd,
            Decimal::from(7050)
        );

        let mut aave = Position::new(
            Chain::Ethereum,
            "aave-v3",
            PositionKind::Lending,
            "USDC".into(),
        );
        aave.tokens.push(PositionToken::new(
            Chain::Ethereum,
            "0xusdc",
            "USDC".into(),
            TokenRole::Supplied,
            Decimal::from(1000),
        ));
        aave.tokens.push(PositionToken::new(
            Chain::Ethereum,
            "0xusdc",
            "USDC".into(),
            TokenRole::Borrowed,
            Decimal::from(400),
        ));
        aave.wallet_id = wallet.id;
        aave.receipts.push(a_usdc.id());
        let fetch = DefiFetch {
            fetched: vec![SourcePositions {
                wallet_id: wallet.id,
                source: "aave-v3:ethereum".into(),
                positions: vec![aave],
            }],
            failures: Vec::new(),
        };
        save(&mut conn, fetch, None).unwrap();
        conn.execute(
            "INSERT INTO token_flags (asset_id, spam, reasons, checked_at) VALUES (?1, 1, '[]', 0)",
            [spam.id()],
        )
        .unwrap();

        let positions = list(&conn).unwrap();
        assert_eq!(positions.len(), 2);
        let lido = positions.iter().find(|p| p.protocol == "lido").unwrap();
        assert!(lido.in_balances);
        assert_eq!(lido.value_usd, Some(Decimal::from(6000)));
        let aave = positions.iter().find(|p| p.protocol == "aave-v3").unwrap();
        assert_eq!(aave.value_usd, Some(Decimal::from(600)));

        let portfolio = repo::portfolio(&conn).unwrap();
        // stETH counted once, aUSDC replaced by the net Aave value, spam hidden.
        assert_eq!(portfolio.total_usd, Decimal::from(6600));
        assert_eq!(portfolio.defi_usd, Decimal::from(600));
        assert_eq!(portfolio.spam_count, 1);
        assert!(
            portfolio
                .assets
                .iter()
                .all(|a| a.id != a_usdc.id() && a.id != spam.id())
        );
        assert_eq!(portfolio.wallets[0].total_usd, Decimal::from(6600));
        assert_eq!(portfolio.chains[0].value_usd, Decimal::from(6600));

        // A refresh of the source with nothing open clears it.
        let fetch = DefiFetch {
            fetched: vec![SourcePositions {
                wallet_id: wallet.id,
                source: "aave-v3:ethereum".into(),
                positions: Vec::new(),
            }],
            failures: Vec::new(),
        };
        save(&mut conn, fetch, None).unwrap();
        assert_eq!(list(&conn).unwrap().len(), 1);
    }
}
