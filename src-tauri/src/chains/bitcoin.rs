//! Bitcoin: address balances from mempool.space (Blockstream as fallback). Extended public keys
//! (xpub/ypub/zpub) are expanded locally and scanned with a gap limit.

use std::time::Duration;

use async_trait::async_trait;
use bitcoin::{
    Address, Network, base58,
    bip32::{ChildNumber, Xpub},
    secp256k1::Secp256k1,
};
use futures::future::try_join_all;
use rust_decimal::Decimal;
use serde::Deserialize;

use super::{AssetMeta, Chain, ChainAdapter, Holding};
use crate::error::{Error, Result};

/// Esplora-compatible APIs, tried in turn.
const APIS: [&str; 2] = ["https://mempool.space/api", "https://blockstream.info/api"];

/// Consecutive unused addresses after which a derivation chain is considered
/// exhausted (BIP44).
pub const GAP_LIMIT: u32 = 20;
/// Hard cap per chain so a pathological key can't trigger endless requests.
const MAX_INDEX: u32 = 1_000;
/// Addresses queried concurrently while scanning. mempool.space rate-limits
/// bursts, so this stays small and 429s are retried with backoff.
const BATCH: u32 = 4;
const RETRIES: u32 = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScriptType {
    /// `xpub` → legacy `1…` addresses (BIP44).
    P2pkh,
    /// `ypub` → nested SegWit `3…` addresses (BIP49).
    P2shP2wpkh,
    /// `zpub` → native SegWit `bc1q…` addresses (BIP84).
    P2wpkh,
}

const XPUB_VERSION: [u8; 4] = [0x04, 0x88, 0xb2, 0x1e];
const YPUB_VERSION: [u8; 4] = [0x04, 0x9d, 0x7c, 0xb2];
const ZPUB_VERSION: [u8; 4] = [0x04, 0xb2, 0x47, 0x46];

pub fn is_extended_key(input: &str) -> bool {
    ["xpub", "ypub", "zpub"]
        .iter()
        .any(|p| input.starts_with(p))
}

/// Parses a mainnet xpub/ypub/zpub into a standard `Xpub` plus the script type
/// its version bytes imply.
pub fn parse_extended_key(input: &str) -> Result<(Xpub, ScriptType)> {
    let mut data = base58::decode_check(input).map_err(|_| Error::InvalidAddress)?;
    if data.len() != 78 {
        return Err(Error::InvalidAddress);
    }
    let version: [u8; 4] = data[..4].try_into().map_err(|_| Error::InvalidAddress)?;
    let script = match version {
        XPUB_VERSION => ScriptType::P2pkh,
        YPUB_VERSION => ScriptType::P2shP2wpkh,
        ZPUB_VERSION => ScriptType::P2wpkh,
        _ => return Err(Error::InvalidAddress),
    };
    data[..4].copy_from_slice(&XPUB_VERSION);
    let xpub = Xpub::decode(&data).map_err(|_| Error::InvalidAddress)?;
    Ok((xpub, script))
}

/// Address at `<key>/<change>/<index>`.
pub fn derive_address(xpub: &Xpub, script: ScriptType, change: u32, index: u32) -> Result<Address> {
    let secp = Secp256k1::verification_only();
    let path = [
        ChildNumber::from_normal_idx(change).map_err(|_| Error::InvalidAddress)?,
        ChildNumber::from_normal_idx(index).map_err(|_| Error::InvalidAddress)?,
    ];
    let child = xpub
        .derive_pub(&secp, &path)
        .map_err(|_| Error::InvalidAddress)?;
    let key = child.to_pub();
    Ok(match script {
        ScriptType::P2pkh => Address::p2pkh(key, Network::Bitcoin),
        ScriptType::P2shP2wpkh => Address::p2shwpkh(&key, Network::Bitcoin),
        ScriptType::P2wpkh => Address::p2wpkh(&key, Network::Bitcoin),
    })
}

#[derive(Debug, Default, Deserialize)]
struct Stats {
    funded_txo_sum: u64,
    spent_txo_sum: u64,
    tx_count: u64,
}

#[derive(Debug, Deserialize)]
struct AddressInfo {
    chain_stats: Stats,
    #[serde(default)]
    mempool_stats: Stats,
}

impl AddressInfo {
    fn used(&self) -> bool {
        self.chain_stats.tx_count + self.mempool_stats.tx_count > 0
    }

    /// Confirmed plus unconfirmed balance, in satoshis.
    fn balance(&self) -> i128 {
        let sum = |s: &Stats| i128::from(s.funded_txo_sum) - i128::from(s.spent_txo_sum);
        sum(&self.chain_stats) + sum(&self.mempool_stats)
    }
}

/// An address derived from an extended key that has on-chain history.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UsedAddress {
    pub address: String,
    pub derivation: String,
}

pub struct BitcoinAdapter {
    http: reqwest::Client,
}

impl BitcoinAdapter {
    pub fn new(http: reqwest::Client) -> Self {
        Self { http }
    }

    /// Address stats, rotating between Esplora instances and backing off when
    /// one rate-limits or drops the connection.
    async fn address_info(&self, address: &str) -> Result<AddressInfo> {
        // Spread addresses across hosts so neither sees the whole burst.
        let offset = address.bytes().map(usize::from).sum::<usize>();
        let mut attempt = 0;
        loop {
            let api = APIS[(offset + attempt as usize) % APIS.len()];
            let response = self
                .http
                .get(format!("{api}/address/{address}"))
                .send()
                .await;
            let retryable = match &response {
                Ok(r) => {
                    r.status() == reqwest::StatusCode::TOO_MANY_REQUESTS
                        || r.status().is_server_error()
                }
                Err(e) => e.is_connect() || e.is_timeout() || e.is_request(),
            };
            if retryable && attempt < RETRIES {
                attempt += 1;
                tokio::time::sleep(Duration::from_millis(500 * 2u64.pow(attempt))).await;
                continue;
            }
            return Ok(response?.error_for_status()?.json().await?);
        }
    }

    /// Walks the receive (0) and change (1) chains until `GAP_LIMIT`
    /// consecutive unused addresses, returning the total in satoshis and the
    /// addresses that have history.
    pub async fn scan_extended_key(&self, input: &str) -> Result<(i128, Vec<UsedAddress>)> {
        let (xpub, script) = parse_extended_key(input)?;
        let mut total = 0i128;
        let mut used = Vec::new();
        for change in [0, 1] {
            let mut next = 0;
            let mut last_used: Option<u32> = None;
            while next < MAX_INDEX && next - last_used.map_or(0, |i| i + 1) < GAP_LIMIT {
                let indices: Vec<u32> = (next..(next + BATCH).min(MAX_INDEX)).collect();
                let addresses = indices
                    .iter()
                    .map(|&i| derive_address(&xpub, script, change, i).map(|a| a.to_string()))
                    .collect::<Result<Vec<_>>>()?;
                let infos = try_join_all(addresses.iter().map(|a| self.address_info(a))).await?;
                for ((index, address), info) in indices.iter().zip(addresses).zip(infos) {
                    if info.used() {
                        last_used = Some(*index);
                        total += info.balance();
                        used.push(UsedAddress {
                            address,
                            derivation: format!("{change}/{index}"),
                        });
                    }
                }
                next += indices.len() as u32;
            }
        }
        Ok((total, used))
    }
}

pub fn btc_holding(sats: i128) -> Option<Holding> {
    (sats > 0).then(|| Holding {
        asset: AssetMeta::native(Chain::Bitcoin, "BTC", "Bitcoin", 8, "bitcoin"),
        amount: Decimal::from_i128_with_scale(sats, 8).normalize(),
        locked: None,
    })
}

#[async_trait]
impl ChainAdapter for BitcoinAdapter {
    fn chain(&self) -> Chain {
        Chain::Bitcoin
    }

    async fn balances(&self, address: &str) -> Result<Vec<Holding>> {
        let sats = if is_extended_key(address) {
            self.scan_extended_key(address).await?.0
        } else {
            self.address_info(address).await?.balance()
        };
        Ok(btc_holding(sats).into_iter().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // BIP84 test vector (mnemonic "abandon … about", account 0).
    const ZPUB: &str = "zpub6rFR7y4Q2AijBEqTUquhVz398htDFrtymD9xYYfG1m4wAcvPhXNfE3EfH1r1ADqtfSdVCToUG868RvUUkgDKf31mGDtKsAYz2oz2AGutZYs";

    #[test]
    fn derives_bip84_addresses_from_zpub() {
        let (xpub, script) = parse_extended_key(ZPUB).unwrap();
        assert_eq!(script, ScriptType::P2wpkh);
        assert_eq!(
            derive_address(&xpub, script, 0, 0).unwrap().to_string(),
            "bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu"
        );
        assert_eq!(
            derive_address(&xpub, script, 0, 1).unwrap().to_string(),
            "bc1qnjg0jd8228aq7egyzacy8cys3knf9xvrerkf9g"
        );
        assert_eq!(
            derive_address(&xpub, script, 1, 0).unwrap().to_string(),
            "bc1q8c6fshw2dlwun7ekn9qwf37cu2rn755upcp6el"
        );
    }

    #[test]
    fn rejects_garbage_keys() {
        assert!(parse_extended_key("xpub123").is_err());
        assert!(parse_extended_key(&ZPUB.replace('r', "s")).is_err());
    }

    #[test]
    fn balance_includes_mempool() {
        let body = r#"{"address": "bc1q", "chain_stats": {"funded_txo_count": 2,
          "funded_txo_sum": 150000, "spent_txo_count": 1, "spent_txo_sum": 50000, "tx_count": 3},
          "mempool_stats": {"funded_txo_count": 1, "funded_txo_sum": 1000, "spent_txo_count": 0,
          "spent_txo_sum": 0, "tx_count": 1}}"#;
        let info: AddressInfo = serde_json::from_str(body).unwrap();
        assert!(info.used());
        assert_eq!(info.balance(), 101_000);
        assert_eq!(
            btc_holding(info.balance()).unwrap().amount,
            Decimal::new(101, 5)
        );
    }
}
