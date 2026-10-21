//! Smoke tests against the real public endpoints. They need network access and
//! depend on third-party uptime, so they are ignored by default:
//! `cargo test -- --ignored --nocapture`

use rusqlite::Connection;

use crate::{
    address::{self, InputKind},
    chains::{self, Chain, ChainAdapter, Family, evm, solana},
    db::{
        self,
        repo::{self, NewWallet},
    },
    state::AppState,
    sync,
};

const VITALIK: &str = "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045";
const ZPUB: &str = "zpub6rFR7y4Q2AijBEqTUquhVz398htDFrtymD9xYYfG1m4wAcvPhXNfE3EfH1r1ADqtfSdVCToUG868RvUUkgDKf31mGDtKsAYz2oz2AGutZYs";
/// An active TRON account holding TRX and USDT (found via USDT transfers).
const TRON: &str = "TUH23i3QneKAb4xTSsw9QF8hp197T3Zpnt";
/// An XRPL account with an RLUSD trust line (found via the issuer's lines).
const XRPL: &str = "rMwNibdiFaEzsTaFCG1NnmAM3Rv3vHUy5L";
/// Solana Foundation's public stake/treasury-style wallet often used in docs.
const SOLANA: &str = "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM";

fn state() -> AppState {
    AppState::new(db::open_in_memory().unwrap()).unwrap()
}

#[tokio::test]
#[ignore = "network"]
async fn resolves_vitalik_eth() {
    let state = state();
    let address = evm::resolve_ens(&state.http, "vitalik.eth").await.unwrap();
    assert_eq!(address, VITALIK);
}

#[tokio::test]
#[ignore = "network"]
async fn resolves_bonfida_sol() {
    let state = state();
    let owner = solana::resolve_sns(&state.http, "bonfida.sol")
        .await
        .unwrap();
    assert!(address::is_solana_address(&owner), "{owner}");
}

#[tokio::test]
#[ignore = "network"]
async fn fetches_every_evm_chain() {
    let state = state();
    for adapter in chains::adapters(Family::Evm, &state.http) {
        let holdings = adapter.balances(VITALIK).await;
        match &holdings {
            Ok(h) => println!("{}: {} holdings", adapter.chain(), h.len()),
            Err(e) => println!("{}: {e}", adapter.chain()),
        }
        assert!(holdings.is_ok(), "{}", adapter.chain());
    }
}

#[tokio::test]
#[ignore = "network"]
async fn curated_tokens_have_correct_decimals() {
    let state = state();
    let mut wrong = Vec::new();
    for chain in Chain::EVM {
        let adapter = evm::EvmAdapter::new(chain, state.http.clone());
        for (symbol, expected, onchain) in adapter.curated_decimals().await.unwrap() {
            if onchain != Some(expected) {
                wrong.push(format!(
                    "{chain} {symbol}: expected {expected}, got {onchain:?}"
                ));
            }
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}

#[tokio::test]
#[ignore = "network"]
async fn scans_test_zpub() {
    let state = state();
    let adapter = chains::bitcoin::BitcoinAdapter::new(state.http.clone());
    let (sats, used) = adapter.scan_extended_key(ZPUB).await.unwrap();
    println!("zpub: {sats} sats across {} used addresses", used.len());
    // The BIP84 test wallet is public and has been used by many people.
    assert!(!used.is_empty());
    assert_eq!(used[0].derivation, "0/0");
}

#[tokio::test]
#[ignore = "network"]
async fn fetches_tron_balances() {
    let state = state();
    let holdings = chains::tron::TronAdapter::new(state.http.clone())
        .balances(TRON)
        .await
        .unwrap();
    for h in &holdings {
        println!(
            "  {:?} {} ({})",
            h.asset.symbol, h.amount, h.asset.price_key
        );
    }
    assert!(
        holdings
            .iter()
            .any(|h| h.asset.symbol.as_deref() == Some("TRX"))
    );
    assert!(
        holdings
            .iter()
            .any(|h| h.asset.symbol.as_deref() == Some("USDT"))
    );
}

#[tokio::test]
#[ignore = "network"]
async fn fetches_xrpl_balances() {
    let state = state();
    let holdings = chains::xrpl::XrplAdapter::new(state.http.clone())
        .balances(XRPL)
        .await
        .unwrap();
    for h in &holdings {
        println!(
            "  {:?} {} locked {:?} ({})",
            h.asset.symbol, h.amount, h.locked, h.asset.price_key
        );
    }
    let xrp = holdings
        .iter()
        .find(|h| h.asset.symbol.as_deref() == Some("XRP"))
        .unwrap();
    assert!(xrp.locked.is_some_and(|l| l > rust_decimal::Decimal::ZERO));
    assert!(
        holdings
            .iter()
            .any(|h| h.asset.symbol.as_deref() == Some("RLUSD"))
    );
}

fn add(conn: &mut Connection, family: Family, kind: InputKind, address: &str) -> i64 {
    repo::insert_wallet(
        conn,
        &NewWallet {
            label: "Live",
            family,
            kind: kind.into(),
            address,
            input: address,
        },
    )
    .unwrap()
    .id
}

#[tokio::test]
#[ignore = "network"]
async fn full_refresh_prices_every_family() {
    let state = state();
    {
        let mut db = state.db();
        add(&mut db, Family::Evm, InputKind::Address, VITALIK);
        add(&mut db, Family::Solana, InputKind::Address, SOLANA);
        add(&mut db, Family::Bitcoin, InputKind::Xpub, ZPUB);
        add(&mut db, Family::Tron, InputKind::Address, TRON);
        add(&mut db, Family::Xrpl, InputKind::Address, XRPL);
    }
    let report = sync::refresh(&state, None).await.unwrap();
    for issue in &report.issues {
        println!("issue: {issue:?}");
    }
    let p = report.portfolio;
    println!(
        "total ${} ({} assets, {} unpriced)",
        p.total_usd,
        p.assets.len(),
        p.unpriced_count
    );
    for asset in p.assets.iter().take(15) {
        println!(
            "  {:<10} {:<9} {} × ${} = ${}",
            asset.symbol, asset.chain, asset.amount, asset.price_usd, asset.value_usd
        );
    }
    for wallet in &p.wallets {
        println!(
            "  wallet {} {}: ${}",
            wallet.id,
            wallet.family.as_str(),
            wallet.total_usd
        );
    }
    assert!(p.total_usd > rust_decimal::Decimal::ZERO);
    assert!(p.assets.iter().any(|a| a.chain == Chain::Ethereum));
    assert!(p.assets.iter().any(|a| a.chain == Chain::Tron));
    // The XRPL account's RLUSD is dust, so only XRP (with its reserve) is listed.
    assert!(
        p.assets
            .iter()
            .any(|a| a.chain == Chain::Xrp && a.locked.is_some())
    );
    let used = state
        .db()
        .query_row(
            "SELECT COUNT(*) FROM wallet_addresses WHERE derivation IS NOT NULL",
            [],
            |r| r.get::<_, i64>(0),
        )
        .unwrap();
    println!("xpub used addresses: {used}");
}
