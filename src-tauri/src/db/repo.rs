//! Queries over the wallet, balance and price tables.

use std::{
    cmp::Reverse,
    collections::{BTreeMap, BTreeSet, HashMap},
    str::FromStr,
};

use rusqlite::{Connection, ErrorCode, OptionalExtension, params};
use rust_decimal::Decimal;
use serde::Serialize;

use super::now_ms;
use crate::{
    address::InputKind,
    chains::{AssetMeta, Chain, Family, Holding, bitcoin::UsedAddress},
    error::{Error, Result},
    prices::Quote,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum WalletKind {
    Address,
    Xpub,
}

impl WalletKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Address => "address",
            Self::Xpub => "xpub",
        }
    }
}

impl From<InputKind> for WalletKind {
    fn from(kind: InputKind) -> Self {
        match kind {
            InputKind::Xpub => Self::Xpub,
            InputKind::Address | InputKind::Ens | InputKind::Sns => Self::Address,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Wallet {
    pub id: i64,
    pub label: String,
    pub family: Family,
    pub kind: WalletKind,
    pub address: String,
    pub input: String,
    pub created_at: i64,
    pub synced_at: Option<i64>,
}

pub struct NewWallet<'a> {
    pub label: &'a str,
    pub family: Family,
    pub kind: WalletKind,
    pub address: &'a str,
    pub input: &'a str,
}

fn wallet_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Wallet> {
    let family: String = row.get(2)?;
    let kind: String = row.get(3)?;
    Ok(Wallet {
        id: row.get(0)?,
        label: row.get(1)?,
        family: Family::from_str(&family).unwrap_or(Family::Evm),
        kind: if kind == "xpub" {
            WalletKind::Xpub
        } else {
            WalletKind::Address
        },
        address: row.get(4)?,
        input: row.get(5)?,
        created_at: row.get(6)?,
        synced_at: row.get(7)?,
    })
}

const WALLET_COLUMNS: &str = "id, label, family, kind, address, input, created_at, synced_at";

pub fn insert_wallet(conn: &mut Connection, wallet: &NewWallet<'_>) -> Result<Wallet> {
    let label = wallet.label.trim();
    if label.is_empty() {
        return Err(Error::EmptyLabel);
    }
    let tx = conn.transaction()?;
    let inserted = tx.execute(
        "INSERT INTO wallets (label, family, kind, address, input, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            label,
            wallet.family.as_str(),
            wallet.kind.as_str(),
            wallet.address,
            wallet.input,
            now_ms()
        ],
    );
    match inserted {
        Err(rusqlite::Error::SqliteFailure(err, _))
            if err.code == ErrorCode::ConstraintViolation =>
        {
            return Err(Error::Duplicate);
        }
        other => other?,
    };
    let id = tx.last_insert_rowid();
    tx.execute(
        "INSERT INTO wallet_addresses (wallet_id, address) VALUES (?1, ?2)",
        params![id, wallet.address],
    )?;
    tx.commit()?;
    get_wallet(conn, id)
}

pub fn get_wallet(conn: &Connection, id: i64) -> Result<Wallet> {
    conn.query_row(
        &format!("SELECT {WALLET_COLUMNS} FROM wallets WHERE id = ?1"),
        [id],
        wallet_from_row,
    )
    .optional()?
    .ok_or(Error::NotFound)
}

pub fn list_wallets(conn: &Connection) -> Result<Vec<Wallet>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {WALLET_COLUMNS} FROM wallets ORDER BY created_at, id"
    ))?;
    let rows = stmt.query_map([], wallet_from_row)?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

pub fn rename_wallet(conn: &Connection, id: i64, label: &str) -> Result<()> {
    let label = label.trim();
    if label.is_empty() {
        return Err(Error::EmptyLabel);
    }
    match conn.execute(
        "UPDATE wallets SET label = ?1 WHERE id = ?2",
        params![label, id],
    )? {
        0 => Err(Error::NotFound),
        _ => Ok(()),
    }
}

pub fn remove_wallet(conn: &Connection, id: i64) -> Result<()> {
    match conn.execute("DELETE FROM wallets WHERE id = ?1", [id])? {
        0 => Err(Error::NotFound),
        _ => Ok(()),
    }
}

/// Fresh holdings of one wallet on one chain; replaces what was stored.
pub struct ChainBalances {
    pub wallet_id: i64,
    pub chain: Chain,
    pub holdings: Vec<Holding>,
}

/// Everything one sync run learned, written in a single transaction.
#[derive(Default)]
pub struct SyncWrite {
    pub balances: Vec<ChainBalances>,
    pub used_addresses: Vec<(i64, UsedAddress)>,
    pub quotes: HashMap<String, Quote>,
    /// Wallets with at least one chain refreshed successfully.
    pub synced_wallets: BTreeSet<i64>,
}

pub fn save_sync(conn: &mut Connection, write: &SyncWrite) -> Result<()> {
    let now = now_ms();
    let tx = conn.transaction()?;
    for chain_balances in &write.balances {
        tx.execute(
            "DELETE FROM balances WHERE wallet_id = ?1
               AND asset_id IN (SELECT id FROM assets WHERE chain = ?2)",
            params![chain_balances.wallet_id, chain_balances.chain.as_str()],
        )?;
        for holding in &chain_balances.holdings {
            upsert_asset(
                &tx,
                &holding.asset,
                write.quotes.get(&holding.asset.price_key),
            )?;
            tx.execute(
                "INSERT INTO balances (wallet_id, asset_id, amount, locked, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT (wallet_id, asset_id) DO UPDATE SET amount = excluded.amount,
                   locked = excluded.locked, updated_at = excluded.updated_at",
                params![
                    chain_balances.wallet_id,
                    holding.asset.id(),
                    holding.amount.to_string(),
                    holding.locked.map(|l| l.to_string()),
                    now
                ],
            )?;
        }
    }
    for (wallet_id, used) in &write.used_addresses {
        tx.execute(
            "INSERT INTO wallet_addresses (wallet_id, address, derivation) VALUES (?1, ?2, ?3)
             ON CONFLICT (wallet_id, address) DO NOTHING",
            params![wallet_id, used.address, used.derivation],
        )?;
    }
    for (key, quote) in &write.quotes {
        tx.execute(
            "INSERT INTO prices (price_key, price_usd, change_24h, updated_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT (price_key) DO UPDATE SET price_usd = excluded.price_usd,
               change_24h = excluded.change_24h, updated_at = excluded.updated_at",
            params![
                key,
                quote.price_usd.to_string(),
                quote.change_24h.map(|c| c.to_string()),
                now
            ],
        )?;
    }
    for wallet_id in &write.synced_wallets {
        tx.execute(
            "UPDATE wallets SET synced_at = ?1 WHERE id = ?2",
            params![now, wallet_id],
        )?;
    }
    tx.commit()?;
    Ok(())
}

/// Inserts or refreshes an asset. A provider symbol wins over the price
/// provider's; when neither exists the contract is shortened into one.
pub(crate) fn upsert_asset(
    tx: &rusqlite::Transaction<'_>,
    asset: &AssetMeta,
    quote: Option<&Quote>,
) -> Result<()> {
    let fallback = || {
        let contract = asset.contract.as_deref().unwrap_or("?");
        contract.chars().take(6).collect::<String>()
    };
    let symbol = asset
        .symbol
        .clone()
        .or_else(|| quote.and_then(|q| q.symbol.as_deref()).map(tidy_symbol))
        .unwrap_or_else(fallback);
    let name = asset.name.clone().unwrap_or_else(|| symbol.clone());
    tx.execute(
        "INSERT INTO assets (id, chain, contract, symbol, name, decimals, price_key, logo_url)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
         ON CONFLICT (id) DO UPDATE SET symbol = excluded.symbol, name = excluded.name,
           decimals = excluded.decimals, price_key = excluded.price_key,
           logo_url = COALESCE(excluded.logo_url, assets.logo_url)",
        params![
            asset.id(),
            asset.chain.as_str(),
            asset.contract,
            symbol,
            name,
            asset.decimals,
            asset.price_key,
            asset.logo_url
        ],
    )?;
    Ok(())
}

/// DefiLlama reports some symbols in lowercase ("wif", "ray"); tickers read
/// better uppercased.
fn tidy_symbol(symbol: &str) -> String {
    if symbol.chars().any(char::is_uppercase) {
        symbol.to_string()
    } else {
        symbol.to_uppercase()
    }
}

/// Every DefiLlama key of an asset some wallet currently holds.
pub fn held_price_keys(conn: &Connection) -> Result<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT DISTINCT a.price_key FROM balances b JOIN assets a ON a.id = b.asset_id",
    )?;
    let rows = stmt.query_map([], |row| row.get(0))?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletSummary {
    pub id: i64,
    pub label: String,
    pub family: Family,
    pub kind: WalletKind,
    pub address: String,
    pub input: String,
    pub created_at: i64,
    pub synced_at: Option<i64>,
    pub total_usd: Decimal,
    /// Chains where this wallet holds priced assets, largest first.
    pub chains: Vec<Chain>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PortfolioAsset {
    pub id: String,
    pub chain: Chain,
    pub symbol: String,
    pub name: String,
    pub logo_url: Option<String>,
    pub amount: Decimal,
    /// Part of `amount` that can't be spent (XRP Ledger reserve).
    pub locked: Option<Decimal>,
    pub price_usd: Decimal,
    pub value_usd: Decimal,
    pub change_24h: Option<Decimal>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChainTotal {
    pub chain: Chain,
    pub value_usd: Decimal,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Portfolio {
    pub total_usd: Decimal,
    pub change_24h_usd: Decimal,
    /// Ratio of the 24h change relative to the value 24h ago.
    pub change_24h: Option<Decimal>,
    /// Priced holdings aggregated across wallets, by value (dust excluded).
    pub assets: Vec<PortfolioAsset>,
    pub chains: Vec<ChainTotal>,
    pub wallets: Vec<WalletSummary>,
    /// Held tokens with no trustworthy price, most of them spam.
    pub unpriced_count: usize,
    pub synced_at: Option<i64>,
}

/// Holdings worth less than this (USD) are left out of the asset list.
const DUST_USD: Decimal = Decimal::from_parts(1, 0, 0, false, 2);

struct BalanceRow {
    wallet_id: i64,
    asset_id: String,
    chain: Chain,
    symbol: String,
    name: String,
    logo_url: Option<String>,
    amount: Decimal,
    locked: Option<Decimal>,
    price: Option<Decimal>,
    change: Option<Decimal>,
}

pub(crate) fn decimal_column(value: Option<String>) -> Option<Decimal> {
    value.and_then(|v| Decimal::from_str(&v).ok())
}

fn balance_rows(conn: &Connection) -> Result<Vec<BalanceRow>> {
    let mut stmt = conn.prepare(
        "SELECT b.wallet_id, a.id, a.chain, a.symbol, a.name, a.logo_url, b.amount,
                p.price_usd, p.change_24h, b.locked
         FROM balances b
         JOIN assets a ON a.id = b.asset_id
         LEFT JOIN prices p ON p.price_key = a.price_key",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, Option<String>>(5)?,
            row.get::<_, String>(6)?,
            row.get::<_, Option<String>>(7)?,
            row.get::<_, Option<String>>(8)?,
            row.get::<_, Option<String>>(9)?,
        ))
    })?;
    let mut out = Vec::new();
    for row in rows {
        let (wallet_id, asset_id, chain, symbol, name, logo_url, amount, price, change, locked) =
            row?;
        let (Ok(chain), Ok(amount)) = (Chain::from_str(&chain), Decimal::from_str(&amount)) else {
            continue;
        };
        out.push(BalanceRow {
            wallet_id,
            asset_id,
            chain,
            symbol,
            name,
            logo_url,
            amount,
            locked: decimal_column(locked),
            price: decimal_column(price),
            change: decimal_column(change),
        });
    }
    Ok(out)
}

/// Current portfolio computed from stored balances and prices.
pub fn portfolio(conn: &Connection) -> Result<Portfolio> {
    let wallets = list_wallets(conn)?;
    let rows = balance_rows(conn)?;

    let mut assets: BTreeMap<String, PortfolioAsset> = BTreeMap::new();
    let mut unpriced = BTreeSet::new();
    let mut wallet_totals: HashMap<i64, Decimal> = HashMap::new();
    let mut wallet_chains: HashMap<i64, BTreeMap<Chain, Decimal>> = HashMap::new();
    let mut chain_totals: BTreeMap<Chain, Decimal> = BTreeMap::new();

    for row in rows {
        let Some(price) = row.price else {
            unpriced.insert(row.asset_id);
            continue;
        };
        let Some(value) = row.amount.checked_mul(price) else {
            continue;
        };
        *wallet_totals.entry(row.wallet_id).or_default() += value;
        *wallet_chains
            .entry(row.wallet_id)
            .or_default()
            .entry(row.chain)
            .or_default() += value;
        *chain_totals.entry(row.chain).or_default() += value;
        let asset = assets
            .entry(row.asset_id.clone())
            .or_insert_with(|| PortfolioAsset {
                id: row.asset_id,
                chain: row.chain,
                symbol: row.symbol,
                name: row.name,
                logo_url: row.logo_url,
                amount: Decimal::ZERO,
                locked: None,
                price_usd: price,
                value_usd: Decimal::ZERO,
                change_24h: row.change,
            });
        asset.amount += row.amount;
        if let Some(locked) = row.locked {
            asset.locked = Some(asset.locked.unwrap_or_default() + locked);
        }
        asset.value_usd += value;
    }

    let total_usd: Decimal = assets.values().map(|a| a.value_usd).sum();
    let change_24h_usd: Decimal = assets
        .values()
        .filter_map(|a| {
            let change = a.change_24h?;
            let previous = Decimal::ONE + change;
            (!previous.is_zero()).then(|| a.value_usd * change / previous)
        })
        .sum();
    let previous_total = total_usd - change_24h_usd;
    let change_24h = (!previous_total.is_zero()).then(|| change_24h_usd / previous_total);

    let mut asset_list: Vec<PortfolioAsset> = assets
        .into_values()
        .filter(|a| a.value_usd >= DUST_USD)
        .map(|mut a| {
            a.amount = a.amount.normalize();
            a.locked = a.locked.map(|l| l.normalize());
            a.value_usd = a.value_usd.round_dp(8).normalize();
            a
        })
        .collect();
    asset_list.sort_by_key(|a| Reverse(a.value_usd));

    let mut chains: Vec<ChainTotal> = chain_totals
        .into_iter()
        .map(|(chain, value_usd)| ChainTotal {
            chain,
            value_usd: value_usd.round_dp(8).normalize(),
        })
        .collect();
    chains.sort_by_key(|c| Reverse(c.value_usd));

    let synced_at = wallets.iter().filter_map(|w| w.synced_at).max();
    let wallets = wallets
        .into_iter()
        .map(|w| {
            let mut by_chain: Vec<(Chain, Decimal)> = wallet_chains
                .remove(&w.id)
                .unwrap_or_default()
                .into_iter()
                .filter(|(_, v)| *v >= DUST_USD)
                .collect();
            by_chain.sort_by_key(|(_, value)| Reverse(*value));
            WalletSummary {
                total_usd: wallet_totals
                    .get(&w.id)
                    .copied()
                    .unwrap_or_default()
                    .round_dp(8)
                    .normalize(),
                chains: by_chain.into_iter().map(|(c, _)| c).collect(),
                id: w.id,
                label: w.label,
                family: w.family,
                kind: w.kind,
                address: w.address,
                input: w.input,
                created_at: w.created_at,
                synced_at: w.synced_at,
            }
        })
        .collect();

    Ok(Portfolio {
        total_usd: total_usd.round_dp(8).normalize(),
        change_24h_usd: change_24h_usd.round_dp(8).normalize(),
        change_24h: change_24h.map(|c| c.round_dp(8).normalize()),
        assets: asset_list,
        chains,
        wallets,
        unpriced_count: unpriced.len(),
        synced_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_in_memory;

    fn add(conn: &mut Connection, family: Family, address: &str) -> Wallet {
        insert_wallet(
            conn,
            &NewWallet {
                label: "Main",
                family,
                kind: WalletKind::Address,
                address,
                input: address,
            },
        )
        .unwrap()
    }

    fn holding(asset: AssetMeta, amount: &str) -> Holding {
        Holding {
            asset,
            amount: Decimal::from_str(amount).unwrap(),
            locked: None,
        }
    }

    fn quote(price: &str, change: Option<&str>) -> Quote {
        Quote {
            price_usd: Decimal::from_str(price).unwrap(),
            change_24h: change.map(|c| Decimal::from_str(c).unwrap()),
            symbol: None,
        }
    }

    fn eth(chain: Chain) -> AssetMeta {
        AssetMeta::native(chain, "ETH", "Ether", 18, "ethereum")
    }

    #[test]
    fn wallet_crud() {
        let mut conn = open_in_memory().unwrap();
        let wallet = add(&mut conn, Family::Evm, "0xabc");
        assert_eq!(wallet.label, "Main");
        assert!(matches!(
            insert_wallet(
                &mut conn,
                &NewWallet {
                    label: "Again",
                    family: Family::Evm,
                    kind: WalletKind::Address,
                    address: "0xabc",
                    input: "0xabc",
                },
            ),
            Err(Error::Duplicate)
        ));

        rename_wallet(&conn, wallet.id, "  Cold  ").unwrap();
        assert_eq!(get_wallet(&conn, wallet.id).unwrap().label, "Cold");
        assert!(matches!(
            rename_wallet(&conn, wallet.id, " "),
            Err(Error::EmptyLabel)
        ));

        remove_wallet(&conn, wallet.id).unwrap();
        assert!(list_wallets(&conn).unwrap().is_empty());
        assert!(matches!(
            remove_wallet(&conn, wallet.id),
            Err(Error::NotFound)
        ));
    }

    #[test]
    fn computes_portfolio_from_balances_and_prices() {
        let mut conn = open_in_memory().unwrap();
        let a = add(&mut conn, Family::Evm, "0xa");
        let b = add(&mut conn, Family::Evm, "0xb");
        let spam = AssetMeta::token(Chain::Base, "0xspam", 18);
        let usdc = AssetMeta::token(Chain::Base, "0xusdc", 6)
            .with_names(Some("USDC".into()), Some("USD Coin".into()));

        let mut write = SyncWrite::default();
        write.balances.push(ChainBalances {
            wallet_id: a.id,
            chain: Chain::Ethereum,
            holdings: vec![holding(eth(Chain::Ethereum), "1.5")],
        });
        write.balances.push(ChainBalances {
            wallet_id: b.id,
            chain: Chain::Base,
            holdings: vec![
                holding(eth(Chain::Base), "0.5"),
                holding(usdc, "100"),
                holding(spam, "1000000"),
            ],
        });
        write
            .quotes
            .insert("coingecko:ethereum".into(), quote("2000", Some("0.25")));
        write.quotes.insert("base:0xusdc".into(), quote("1", None));
        write.synced_wallets.extend([a.id, b.id]);
        save_sync(&mut conn, &write).unwrap();

        let p = portfolio(&conn).unwrap();
        assert_eq!(p.total_usd, Decimal::from(4100));
        // ETH is worth 4000 now, +25% means 3200 a day ago: +800 overall.
        assert_eq!(p.change_24h_usd, Decimal::from(800));
        assert_eq!(
            p.change_24h,
            Some(Decimal::from_str("0.2424242424").unwrap().round_dp(8))
        );
        assert_eq!(p.unpriced_count, 1);
        assert_eq!(p.assets.len(), 3);
        assert_eq!(p.assets[0].id, "ethereum:native");
        assert_eq!(p.assets[0].value_usd, Decimal::from(3000));
        assert_eq!(p.chains[0].chain, Chain::Ethereum);
        assert_eq!(p.chains[1].value_usd, Decimal::from(1100));
        assert_eq!(p.wallets[1].total_usd, Decimal::from(1100));
        assert_eq!(p.wallets[1].chains, vec![Chain::Base]);
        assert!(p.synced_at.is_some());

        // A new sync of one chain replaces only that chain's balances.
        let mut write = SyncWrite::default();
        write.balances.push(ChainBalances {
            wallet_id: b.id,
            chain: Chain::Base,
            holdings: vec![],
        });
        save_sync(&mut conn, &write).unwrap();
        let p = portfolio(&conn).unwrap();
        assert_eq!(p.total_usd, Decimal::from(3000));
        assert_eq!(p.unpriced_count, 0);

        // Locked amounts (XRP reserve) are stored and summed per asset.
        let x = add(&mut conn, Family::Xrpl, "rX");
        let mut write = SyncWrite::default();
        let mut xrp = holding(
            AssetMeta::native(Chain::Xrp, "XRP", "XRP", 6, "ripple"),
            "10",
        );
        xrp.locked = Some(Decimal::from_str("1.2").unwrap());
        write.balances.push(ChainBalances {
            wallet_id: x.id,
            chain: Chain::Xrp,
            holdings: vec![xrp],
        });
        write
            .quotes
            .insert("coingecko:ripple".into(), quote("2", None));
        save_sync(&mut conn, &write).unwrap();
        let p = portfolio(&conn).unwrap();
        let xrp = p.assets.iter().find(|a| a.chain == Chain::Xrp).unwrap();
        assert_eq!(xrp.locked, Some(Decimal::from_str("1.2").unwrap()));
        assert_eq!(xrp.value_usd, Decimal::from(20));
        remove_wallet(&conn, x.id).unwrap();

        // Removing a wallet drops its balances.
        remove_wallet(&conn, a.id).unwrap();
        assert_eq!(portfolio(&conn).unwrap().total_usd, Decimal::ZERO);
        assert!(held_price_keys(&conn).unwrap().is_empty());
    }
}
