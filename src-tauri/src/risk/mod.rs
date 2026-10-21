//! Risk scanner: a 0–100 score (higher is riskier) built from concentration,
//! stablecoin depegs, suspicious tokens, open approvals and chain/protocol
//! exposure. The report is computed from local data; [`scan`] refreshes the
//! parts that need the network (GoPlus verdicts and approvals).

pub mod approvals;
pub mod goplus;
pub mod score;

use std::{
    cmp::Reverse,
    collections::{BTreeMap, HashMap},
    str::FromStr,
    sync::OnceLock,
    time::Duration,
};

use rusqlite::{Connection, params};
use rust_decimal::Decimal;
use serde::Serialize;

use self::score::ExposureShares;
use crate::{
    chains::{Chain, Family},
    db::{
        now_ms,
        repo::{self, WalletKind},
    },
    defi::{self, TokenRole},
    error::Result,
    state::AppState,
    sync::SyncIssue,
};

/// Chains whose approvals can be listed without an API key.
pub const APPROVAL_CHAINS: [Chain; 2] = [Chain::Ethereum, Chain::Optimism];
/// GoPlus checks per scan; keyless access throttles after a handful.
const TOKEN_CHECKS_PER_SCAN: usize = 16;
const TOKEN_CHECK_SPACING: Duration = Duration::from_millis(1200);
/// Token verdicts are reused for this long.
const VERDICT_TTL_MS: i64 = 14 * 24 * 3600 * 1000;

const STABLECOINS: &[&str] = &[
    "USDC", "USDT", "DAI", "USDS", "FDUSD", "PYUSD", "USDE", "TUSD", "FRAX", "LUSD", "GHO",
    "USDC.E", "USDBC", "USDT.E", "USDT0", "DAI.E", "RLUSD", "CRVUSD", "USD1", "USDD", "BUSD",
    "SUSD",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FactorKey {
    Concentration,
    Depeg,
    Suspicious,
    Approvals,
    Exposure,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Factor {
    pub key: FactorKey,
    pub score: u8,
    pub weight: Decimal,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Concentration {
    pub hhi: Decimal,
    pub top_symbol: Option<String>,
    pub top_share: Decimal,
    pub assets: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StableExposure {
    pub symbol: String,
    pub chain: Chain,
    pub price_usd: Decimal,
    pub deviation: Decimal,
    pub value_usd: Decimal,
    pub depegged: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FlaggedToken {
    pub asset_id: String,
    pub chain: Chain,
    pub symbol: String,
    pub name: String,
    pub reasons: Vec<goplus::Reason>,
    /// Hidden from the portfolio.
    pub spam: bool,
    pub value_usd: Option<Decimal>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Approval {
    pub wallet_id: i64,
    pub owner: String,
    pub chain: Chain,
    pub token: String,
    pub token_symbol: Option<String>,
    pub spender: String,
    pub spender_name: Option<String>,
    /// Allowance in whole tokens; `None` when unlimited or the decimals are
    /// unknown.
    pub amount: Option<Decimal>,
    pub unlimited: bool,
    pub revoke_url: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Share<K> {
    pub key: K,
    pub value_usd: Decimal,
    pub share: Decimal,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Exposure {
    pub chains: Vec<Share<Chain>>,
    pub protocols: Vec<Share<String>>,
    pub defi_share: Decimal,
    pub min_health_factor: Option<Decimal>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RiskReport {
    pub score: u8,
    pub factors: Vec<Factor>,
    pub total_usd: Decimal,
    pub concentration: Concentration,
    pub stablecoins: Vec<StableExposure>,
    pub flagged: Vec<FlaggedToken>,
    pub approvals: Vec<Approval>,
    pub approval_chains: Vec<Chain>,
    pub exposure: Exposure,
    /// Last network scan (approvals and token verdicts), if any.
    pub scanned_at: Option<i64>,
}

/// Wrapped tokens count as their underlying asset for concentration.
fn canonical(symbol: &str) -> String {
    let upper = symbol.to_uppercase();
    match upper.as_str() {
        "WETH" | "WETH.E" => "ETH".into(),
        "WBTC" | "CBBTC" | "BTCB" | "BTC.B" => "BTC".into(),
        "WSOL" => "SOL".into(),
        "WPOL" | "WMATIC" | "MATIC" => "POL".into(),
        "WAVAX" => "AVAX".into(),
        "WBNB" => "BNB".into(),
        _ => upper,
    }
}

pub fn is_stablecoin(symbol: &str) -> bool {
    STABLECOINS.contains(&symbol.to_uppercase().as_str())
}

fn share(part: Decimal, total: Decimal) -> Decimal {
    if total.is_zero() {
        Decimal::ZERO
    } else {
        (part / total).round_dp(6).normalize()
    }
}

/// Builds the risk report from what is stored locally.
pub fn report(conn: &Connection) -> Result<RiskReport> {
    let portfolio = repo::portfolio(conn)?;
    let positions = defi::store::list(conn)?;
    let total = portfolio.total_usd;

    // Concentration over holdings plus position assets not in the holdings.
    let mut by_asset: BTreeMap<String, Decimal> = BTreeMap::new();
    for asset in &portfolio.assets {
        *by_asset.entry(canonical(&asset.symbol)).or_default() += asset.value_usd;
    }
    for position in positions.iter().filter(|p| !p.in_balances) {
        for token in position
            .tokens
            .iter()
            .filter(|t| t.role != TokenRole::Borrowed)
        {
            *by_asset.entry(canonical(&token.symbol)).or_default() +=
                token.value_usd.unwrap_or_default();
        }
    }
    let values: Vec<Decimal> = by_asset.values().copied().collect();
    let gross: Decimal = values.iter().sum();
    let top = by_asset.iter().max_by_key(|(_, v)| **v);
    let concentration = Concentration {
        hhi: score::hhi(&values),
        top_symbol: top.map(|(s, _)| s.clone()),
        top_share: top.map_or(Decimal::ZERO, |(_, v)| share(*v, gross)),
        assets: values.iter().filter(|v| !v.is_zero()).count(),
    };

    let stablecoins: Vec<StableExposure> = portfolio
        .assets
        .iter()
        .filter(|a| is_stablecoin(&a.symbol))
        .map(|a| StableExposure {
            symbol: a.symbol.clone(),
            chain: a.chain,
            price_usd: a.price_usd,
            deviation: score::deviation(a.price_usd).round_dp(6).normalize(),
            value_usd: a.value_usd,
            depegged: score::is_depegged(a.price_usd),
        })
        .collect();

    let flagged = flagged_tokens(conn, &portfolio.assets)?;
    let approvals = stored_approvals(conn)?;

    let mut protocols: BTreeMap<String, Decimal> = BTreeMap::new();
    for p in &positions {
        *protocols.entry(p.protocol.clone()).or_default() +=
            p.value_usd.unwrap_or_default().max(Decimal::ZERO);
    }
    let defi_value: Decimal = protocols.values().sum();
    let mut protocols: Vec<Share<String>> = protocols
        .into_iter()
        .map(|(key, value_usd)| Share {
            share: share(value_usd, total),
            key,
            value_usd,
        })
        .collect();
    protocols.sort_by_key(|s| Reverse(s.value_usd));
    let chains: Vec<Share<Chain>> = portfolio
        .chains
        .iter()
        .map(|c| Share {
            key: c.chain,
            value_usd: c.value_usd,
            share: share(c.value_usd, total),
        })
        .collect();
    let exposure = Exposure {
        min_health_factor: positions.iter().filter_map(|p| p.health_factor).min(),
        defi_share: share(defi_value, total).min(Decimal::ONE),
        chains,
        protocols,
    };

    let risky_value: Decimal = flagged
        .iter()
        .filter(|f| !f.spam)
        .filter_map(|f| f.value_usd)
        .sum();
    let unlimited = approvals.iter().filter(|a| a.unlimited).count();
    let factors = vec![
        (
            FactorKey::Concentration,
            score::concentration(concentration.hhi, concentration.top_share),
            Decimal::new(30, 2),
        ),
        (
            FactorKey::Depeg,
            score::depeg(
                &stablecoins
                    .iter()
                    .map(|s| (s.price_usd, s.value_usd))
                    .collect::<Vec<_>>(),
                total,
            ),
            Decimal::new(15, 2),
        ),
        (
            FactorKey::Suspicious,
            score::suspicious(
                flagged.iter().filter(|f| f.spam).count(),
                risky_value,
                total,
            ),
            Decimal::new(15, 2),
        ),
        (
            FactorKey::Approvals,
            score::approvals(unlimited, approvals.len() - unlimited),
            Decimal::new(20, 2),
        ),
        (
            FactorKey::Exposure,
            score::exposure(
                &ExposureShares {
                    defi: exposure.defi_share,
                    top_protocol: exposure
                        .protocols
                        .first()
                        .map_or(Decimal::ZERO, |p| p.share),
                    top_chain: exposure.chains.first().map_or(Decimal::ZERO, |c| c.share),
                },
                exposure.min_health_factor,
            ),
            Decimal::new(20, 2),
        ),
    ];
    let has_holdings = !total.is_zero();
    Ok(RiskReport {
        score: if has_holdings {
            score::overall(&factors.iter().map(|(_, s, w)| (*s, *w)).collect::<Vec<_>>())
        } else {
            0
        },
        factors: factors
            .into_iter()
            .map(|(key, score, weight)| Factor {
                key,
                score: if has_holdings { score } else { 0 },
                weight,
            })
            .collect(),
        total_usd: total,
        concentration,
        stablecoins,
        flagged,
        approvals,
        approval_chains: APPROVAL_CHAINS.to_vec(),
        exposure,
        scanned_at: conn.query_row("SELECT MAX(scanned_at) FROM approval_scans", [], |r| {
            r.get(0)
        })?,
    })
}

fn flagged_tokens(conn: &Connection, held: &[repo::PortfolioAsset]) -> Result<Vec<FlaggedToken>> {
    let mut stmt = conn.prepare(
        "SELECT f.asset_id, f.spam, f.reasons, a.chain, a.symbol, a.name
         FROM token_flags f JOIN assets a ON a.id = f.asset_id
         WHERE f.reasons != '[]' AND EXISTS (SELECT 1 FROM balances b WHERE b.asset_id = f.asset_id)",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, bool>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, String>(3)?,
            r.get::<_, String>(4)?,
            r.get::<_, String>(5)?,
        ))
    })?;
    let values: HashMap<&str, Decimal> =
        held.iter().map(|a| (a.id.as_str(), a.value_usd)).collect();
    let mut out = Vec::new();
    for row in rows {
        let (asset_id, spam, reasons, chain, symbol, name) = row?;
        let Ok(chain) = Chain::from_str(&chain) else {
            continue;
        };
        out.push(FlaggedToken {
            value_usd: values.get(asset_id.as_str()).copied(),
            reasons: serde_json::from_str(&reasons).unwrap_or_default(),
            asset_id,
            chain,
            symbol,
            name,
            spam,
        });
    }
    out.sort_by_key(|f| (Reverse(f.value_usd), f.symbol.clone()));
    Ok(out)
}

fn stored_approvals(conn: &Connection) -> Result<Vec<Approval>> {
    let mut stmt = conn.prepare(
        "SELECT p.wallet_id, w.address, p.chain, p.token, p.token_symbol, p.token_decimals,
                p.spender, p.spender_name, p.amount, p.unlimited
         FROM approvals p JOIN wallets w ON w.id = p.wallet_id
         ORDER BY p.unlimited DESC, p.chain, p.token_symbol",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, String>(3)?,
            r.get::<_, Option<String>>(4)?,
            r.get::<_, Option<u32>>(5)?,
            r.get::<_, String>(6)?,
            r.get::<_, Option<String>>(7)?,
            r.get::<_, String>(8)?,
            r.get::<_, bool>(9)?,
        ))
    })?;
    let mut out = Vec::new();
    for row in rows {
        let (
            wallet_id,
            owner,
            chain,
            token,
            token_symbol,
            decimals,
            spender,
            spender_name,
            amount,
            unlimited,
        ) = row?;
        let Ok(chain) = Chain::from_str(&chain) else {
            continue;
        };
        let amount = match (unlimited, decimals) {
            (false, Some(decimals)) => crate::chains::from_base_units(&amount, decimals),
            _ => None,
        };
        out.push(Approval {
            revoke_url: approvals::revoke_url(&owner, chain),
            wallet_id,
            owner,
            chain,
            token,
            token_symbol,
            spender,
            spender_name,
            amount,
            unlimited,
        });
    }
    Ok(out)
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RiskScan {
    pub report: RiskReport,
    pub issues: Vec<SyncIssue>,
}

/// Held EVM tokens without a fresh verdict, most valuable first.
fn tokens_to_check(conn: &Connection) -> Result<Vec<(String, Chain, String)>> {
    let mut stmt = conn.prepare(
        "SELECT a.id, a.chain, a.contract,
                SUM(CAST(b.amount AS REAL) * COALESCE(CAST(p.price_usd AS REAL), 0)) AS value
         FROM balances b
         JOIN assets a ON a.id = b.asset_id
         LEFT JOIN prices p ON p.price_key = a.price_key
         LEFT JOIN token_flags f ON f.asset_id = a.id
         WHERE a.contract IS NOT NULL AND (f.checked_at IS NULL OR f.checked_at < ?1)
         GROUP BY a.id
         ORDER BY value DESC",
    )?;
    let rows = stmt.query_map([now_ms() - VERDICT_TTL_MS], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
        ))
    })?;
    let mut out = Vec::new();
    for row in rows {
        let (id, chain, contract) = row?;
        if let Ok(chain) = Chain::from_str(&chain) {
            if goplus::chain_id(chain).is_some() {
                out.push((id, chain, contract));
            }
        }
    }
    Ok(out)
}

async fn check_tokens(state: &AppState, issues: &mut Vec<SyncIssue>) -> Result<()> {
    let candidates = tokens_to_check(&state.db())?;
    for (i, (asset_id, chain, contract)) in candidates
        .into_iter()
        .take(TOKEN_CHECKS_PER_SCAN)
        .enumerate()
    {
        if i > 0 {
            tokio::time::sleep(TOKEN_CHECK_SPACING).await;
        }
        let verdict = match goplus::check(&state.http, chain, &contract).await {
            Ok(goplus::Outcome::Checked(verdict)) => verdict.unwrap_or_default(),
            Ok(goplus::Outcome::RateLimited) => break,
            Err(err) => {
                issues.push(SyncIssue {
                    wallet_id: None,
                    chain: Some(chain),
                    message: format!("token security: {err}"),
                });
                break;
            }
        };
        let reasons = serde_json::to_string(&verdict.reasons).unwrap_or_else(|_| "[]".into());
        state.db().execute(
            "INSERT INTO token_flags (asset_id, spam, reasons, checked_at) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT (asset_id) DO UPDATE SET spam = excluded.spam, reasons = excluded.reasons,
               checked_at = excluded.checked_at",
            params![asset_id, verdict.spam, reasons, now_ms()],
        )?;
    }
    Ok(())
}

async fn scan_approvals(state: &AppState, issues: &mut Vec<SyncIssue>) -> Result<()> {
    let wallets = repo::list_wallets(&state.db())?;
    let jobs = wallets
        .iter()
        .filter(|w| w.family == Family::Evm && w.kind == WalletKind::Address)
        .flat_map(|w| APPROVAL_CHAINS.map(|chain| (w, chain)));
    for (wallet, chain) in jobs {
        match approvals::scan(&state.http, chain, &wallet.address).await {
            Ok(open) => {
                let mut db = state.db();
                let tx = db.transaction()?;
                tx.execute(
                    "DELETE FROM approvals WHERE wallet_id = ?1 AND chain = ?2",
                    params![wallet.id, chain.as_str()],
                )?;
                let now = now_ms();
                for a in open {
                    tx.execute(
                        "INSERT OR REPLACE INTO approvals (wallet_id, chain, token, spender, token_symbol,
                           token_decimals, spender_name, amount, unlimited, updated_at)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                        params![
                            wallet.id,
                            chain.as_str(),
                            a.token,
                            a.spender,
                            a.token_symbol,
                            a.token_decimals,
                            a.spender_name,
                            a.amount.to_string(),
                            a.unlimited,
                            now
                        ],
                    )?;
                }
                tx.execute(
                    "INSERT INTO approval_scans (wallet_id, chain, scanned_at) VALUES (?1, ?2, ?3)
                     ON CONFLICT (wallet_id, chain) DO UPDATE SET scanned_at = excluded.scanned_at",
                    params![wallet.id, chain.as_str(), now],
                )?;
                tx.commit()?;
            }
            Err(err) => issues.push(SyncIssue {
                wallet_id: Some(wallet.id),
                chain: Some(chain),
                message: format!("approvals: {err}"),
            }),
        }
    }
    Ok(())
}

/// Refreshes token verdicts and approvals, then rebuilds the report.
pub async fn scan(state: &AppState) -> Result<RiskScan> {
    static RUNNING: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();
    let _running = RUNNING.get_or_init(Default::default).lock().await;
    let (mut token_issues, mut approval_issues) = (Vec::new(), Vec::new());
    let (tokens, approvals) = tokio::join!(
        check_tokens(state, &mut token_issues),
        scan_approvals(state, &mut approval_issues)
    );
    tokens?;
    approvals?;
    token_issues.extend(approval_issues);
    Ok(RiskScan {
        report: report(&state.db())?,
        issues: token_issues,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        chains::{AssetMeta, Holding},
        db::{
            open_in_memory,
            repo::{ChainBalances, NewWallet, SyncWrite},
        },
        prices::Quote,
    };

    #[test]
    fn reports_risk_from_local_data() {
        let mut conn = open_in_memory().unwrap();
        let empty = report(&conn).unwrap();
        assert_eq!(empty.score, 0);
        assert!(empty.factors.iter().all(|f| f.score == 0));

        let wallet = repo::insert_wallet(
            &mut conn,
            &NewWallet {
                label: "Main",
                family: Family::Evm,
                kind: WalletKind::Address,
                address: "0xd8da6bf26964af9d7eed9e03e53415d37aa96045",
                input: "vitalik.eth",
            },
        )
        .unwrap();
        let eth = AssetMeta::native(Chain::Ethereum, "ETH", "Ether", 18, "ethereum");
        let weth =
            AssetMeta::token(Chain::Base, "0xweth", 18).with_names(Some("WETH".into()), None);
        let usdc =
            AssetMeta::token(Chain::Ethereum, "0xusdc", 6).with_names(Some("USDC".into()), None);
        let usde =
            AssetMeta::token(Chain::Ethereum, "0xusde", 18).with_names(Some("USDe".into()), None);
        let tax =
            AssetMeta::token(Chain::Ethereum, "0xtax", 18).with_names(Some("TAX".into()), None);
        let mut write = SyncWrite::default();
        for (chain, holdings) in [
            (
                Chain::Ethereum,
                vec![
                    (eth, 1),
                    (usdc.clone(), 1000),
                    (usde.clone(), 1000),
                    (tax.clone(), 100),
                ],
            ),
            (Chain::Base, vec![(weth, 1)]),
        ] {
            write.balances.push(ChainBalances {
                wallet_id: wallet.id,
                chain,
                holdings: holdings
                    .into_iter()
                    .map(|(asset, amount)| Holding {
                        asset,
                        amount: Decimal::from(amount),
                        locked: None,
                    })
                    .collect(),
            });
        }
        for (key, price) in [
            ("coingecko:ethereum", "3000"),
            (usdc.price_key.as_str(), "0.9998"),
            (usde.price_key.as_str(), "0.96"),
            (tax.price_key.as_str(), "1"),
            ("base:0xweth", "3000"),
        ] {
            write.quotes.insert(
                key.into(),
                Quote {
                    price_usd: Decimal::from_str(price).unwrap(),
                    change_24h: None,
                    symbol: None,
                },
            );
        }
        repo::save_sync(&mut conn, &write).unwrap();
        conn.execute(
            "INSERT INTO token_flags VALUES (?1, 0, '[\"highTax\"]', 0)",
            [tax.id()],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO approvals VALUES (?1, 'ethereum', '0xusdc', '0x000000000022d473030f116ddee9f6b43ac78ba3',
               'USDC', 6, 'Uniswap Permit2', ?2, 1, 0)",
            params![wallet.id, U256_MAX],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO approvals VALUES (?1, 'ethereum', '0xusde', '0xspender', 'USDe', 18, NULL,
               '2500000000000000000000', 0, 0)",
            [wallet.id],
        )
        .unwrap();

        let r = report(&conn).unwrap();
        // ETH and WETH merge: 6000 of 8059.8 in one asset.
        assert_eq!(r.concentration.top_symbol.as_deref(), Some("ETH"));
        assert_eq!(r.concentration.assets, 4);
        assert!(r.concentration.top_share > Decimal::new(74, 2));
        let usde = r.stablecoins.iter().find(|s| s.symbol == "USDe").unwrap();
        assert!(usde.depegged);
        assert_eq!(usde.deviation, Decimal::new(4, 2));
        assert!(
            !r.stablecoins
                .iter()
                .find(|s| s.symbol == "USDC")
                .unwrap()
                .depegged
        );
        assert_eq!(r.flagged.len(), 1);
        assert_eq!(r.flagged[0].value_usd, Some(Decimal::from(100)));
        assert_eq!(r.approvals.len(), 2);
        assert!(r.approvals[0].unlimited);
        assert_eq!(r.approvals[0].amount, None);
        assert_eq!(
            r.approvals[0].revoke_url.as_deref(),
            Some(
                "https://revoke.cash/address/0xd8da6bf26964af9d7eed9e03e53415d37aa96045?chainId=1"
            )
        );
        assert_eq!(r.approvals[1].amount, Some(Decimal::from(2500)));
        let factor = |key| r.factors.iter().find(|f| f.key == key).unwrap().score;
        assert_eq!(factor(FactorKey::Approvals), 20);
        assert!(factor(FactorKey::Depeg) > 0);
        assert!(factor(FactorKey::Suspicious) > 0);
        assert!(r.score > 0 && r.score < 100);
        assert_eq!(r.scanned_at, None);
    }

    const U256_MAX: &str =
        "115792089237316195423570985008687907853269984665640564039457584007913129639935";

    #[test]
    fn recognizes_stablecoins_and_wrappers() {
        assert!(is_stablecoin("usdc.e"));
        assert!(is_stablecoin("USDe"));
        assert!(!is_stablecoin("ETH"));
        assert_eq!(canonical("WETH"), "ETH");
        assert_eq!(canonical("cbBTC"), "BTC");
        assert_eq!(canonical("pepe"), "PEPE");
    }
}
