//! Chain adapters: each one turns a watch-only address into token holdings
//! using free, keyless public endpoints.

pub mod bitcoin;
pub mod evm;
mod evm_tokens;
pub(crate) mod rpc;
pub mod solana;
pub mod tron;
pub mod xrpl;

use std::{fmt, str::FromStr, sync::Arc};

use async_trait::async_trait;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// Address families. One wallet belongs to exactly one family.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Family {
    Evm,
    Solana,
    Bitcoin,
    Tron,
    Xrpl,
}

impl Family {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Evm => "evm",
            Self::Solana => "solana",
            Self::Bitcoin => "bitcoin",
            Self::Tron => "tron",
            Self::Xrpl => "xrpl",
        }
    }

    /// Every chain an address of this family is scanned on.
    pub fn chains(self) -> &'static [Chain] {
        match self {
            Self::Evm => &Chain::EVM,
            Self::Solana => &[Chain::Solana],
            Self::Bitcoin => &[Chain::Bitcoin],
            Self::Tron => &[Chain::Tron],
            Self::Xrpl => &[Chain::Xrp],
        }
    }
}

impl FromStr for Family {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "evm" => Ok(Self::Evm),
            "solana" => Ok(Self::Solana),
            "bitcoin" => Ok(Self::Bitcoin),
            "tron" => Ok(Self::Tron),
            "xrpl" => Ok(Self::Xrpl),
            _ => Err(Error::InvalidAddress),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Chain {
    Ethereum,
    Arbitrum,
    Base,
    Optimism,
    Polygon,
    Bsc,
    Avalanche,
    Solana,
    Bitcoin,
    Tron,
    Xrp,
}

impl Chain {
    pub const EVM: [Chain; 7] = [
        Self::Ethereum,
        Self::Arbitrum,
        Self::Base,
        Self::Optimism,
        Self::Polygon,
        Self::Bsc,
        Self::Avalanche,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ethereum => "ethereum",
            Self::Arbitrum => "arbitrum",
            Self::Base => "base",
            Self::Optimism => "optimism",
            Self::Polygon => "polygon",
            Self::Bsc => "bsc",
            Self::Avalanche => "avalanche",
            Self::Solana => "solana",
            Self::Bitcoin => "bitcoin",
            Self::Tron => "tron",
            Self::Xrp => "xrp",
        }
    }

    /// Chain prefix DefiLlama uses in coin ids (`<prefix>:<contract>`).
    pub fn llama_prefix(self) -> &'static str {
        match self {
            Self::Avalanche => "avax",
            Self::Xrp => "xrpl",
            other => other.as_str(),
        }
    }
}

impl fmt::Display for Chain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Chain {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        [Chain::Solana, Chain::Bitcoin, Chain::Tron, Chain::Xrp]
            .into_iter()
            .chain(Chain::EVM)
            .find(|c| c.as_str() == s)
            .ok_or_else(|| Error::provider(format!("unknown chain {s}")))
    }
}

/// Static description of a token on a chain.
#[derive(Clone, Debug, PartialEq)]
pub struct AssetMeta {
    pub chain: Chain,
    /// Contract address or mint; `None` for the chain's native coin.
    pub contract: Option<String>,
    /// Best known symbol. `None` when the provider did not report one; the
    /// price provider can fill it in later.
    pub symbol: Option<String>,
    pub name: Option<String>,
    pub decimals: u32,
    /// DefiLlama coin id, e.g. `coingecko:ethereum` or `base:0x…`.
    pub price_key: String,
    pub logo_url: Option<String>,
}

impl AssetMeta {
    pub fn native(chain: Chain, symbol: &str, name: &str, decimals: u32, gecko_id: &str) -> Self {
        Self {
            chain,
            contract: None,
            symbol: Some(symbol.into()),
            name: Some(name.into()),
            decimals,
            price_key: format!("coingecko:{gecko_id}"),
            logo_url: None,
        }
    }

    pub fn token(chain: Chain, contract: impl Into<String>, decimals: u32) -> Self {
        let contract = contract.into();
        Self {
            chain,
            price_key: format!("{}:{}", chain.llama_prefix(), contract),
            contract: Some(contract),
            symbol: None,
            name: None,
            decimals,
            logo_url: None,
        }
    }

    pub fn with_names(mut self, symbol: Option<String>, name: Option<String>) -> Self {
        self.symbol = symbol.filter(|s| !s.trim().is_empty());
        self.name = name.filter(|s| !s.trim().is_empty());
        self
    }

    /// Primary key of the `assets` table.
    pub fn id(&self) -> String {
        format!(
            "{}:{}",
            self.chain,
            self.contract.as_deref().unwrap_or("native")
        )
    }
}

/// A balance of one asset held by one address.
#[derive(Clone, Debug, PartialEq)]
pub struct Holding {
    pub asset: AssetMeta,
    pub amount: Decimal,
    /// Part of `amount` the owner can't spend, e.g. the XRP Ledger reserve.
    pub locked: Option<Decimal>,
}

/// Fetches the current holdings of a watch-only address on one chain.
#[async_trait]
pub trait ChainAdapter: Send + Sync {
    fn chain(&self) -> Chain;

    /// Non-zero holdings of `address`. For Bitcoin, `address` may also be an
    /// extended public key, in which case all derived addresses are summed.
    async fn balances(&self, address: &str) -> Result<Vec<Holding>>;
}

/// Adapters used for every address of `family`, one per chain.
pub fn adapters(family: Family, http: &reqwest::Client) -> Vec<Arc<dyn ChainAdapter>> {
    match family {
        Family::Evm => Chain::EVM
            .iter()
            .map(|&chain| Arc::new(evm::EvmAdapter::new(chain, http.clone())) as _)
            .collect(),
        Family::Solana => vec![Arc::new(solana::SolanaAdapter::new(http.clone()))],
        Family::Bitcoin => vec![Arc::new(bitcoin::BitcoinAdapter::new(http.clone()))],
        Family::Tron => vec![Arc::new(tron::TronAdapter::new(http.clone()))],
        Family::Xrpl => vec![Arc::new(xrpl::XrplAdapter::new(http.clone()))],
    }
}

/// Converts an integer amount in base units (wei, lamports, sats…) into a
/// decimal. Values with more digits than `Decimal` can hold lose their least
/// significant (dust) digits instead of failing.
pub fn from_base_units(raw: &str, decimals: u32) -> Option<Decimal> {
    let raw = raw.trim();
    if raw.is_empty() || !raw.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let mut digits = raw.trim_start_matches('0').to_string();
    let mut scale = decimals;
    while (digits.len() > 28 || scale > 28) && scale > 0 {
        digits.pop();
        scale -= 1;
    }
    if digits.is_empty() {
        return Some(Decimal::ZERO);
    }
    if digits.len() > 28 {
        return None;
    }
    let mantissa: i128 = digits.parse().ok()?;
    Decimal::try_from_i128_with_scale(mantissa, scale)
        .ok()
        .map(|d| d.normalize())
}

/// Same as [`from_base_units`] for `0x`-prefixed hex quantities (JSON-RPC).
pub fn from_hex_units(hex: &str, decimals: u32) -> Option<Decimal> {
    let digits = hex.trim().trim_start_matches("0x").trim_start_matches('0');
    if digits.is_empty() {
        return Some(Decimal::ZERO);
    }
    let value = u128::from_str_radix(digits, 16).ok()?;
    from_base_units(&value.to_string(), decimals)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_base_units() {
        assert_eq!(
            from_base_units("1500000000000000000", 18),
            Some(Decimal::new(15, 1))
        );
        assert_eq!(from_base_units("0", 18), Some(Decimal::ZERO));
        assert_eq!(from_base_units("123", 0), Some(Decimal::new(123, 0)));
        assert_eq!(from_base_units("12a", 0), None);
    }

    #[test]
    fn huge_raw_values_drop_dust_digits() {
        // 10^11 tokens with 18 decimals has 30 digits, more than Decimal holds.
        let raw = format!("1{}", "0".repeat(29));
        assert_eq!(
            from_base_units(&raw, 18),
            Some(Decimal::new(100_000_000_000, 0))
        );
    }

    #[test]
    fn converts_hex_units() {
        assert_eq!(from_hex_units("0xde0b6b3a7640000", 18), Some(Decimal::ONE));
        assert_eq!(from_hex_units("0x0", 18), Some(Decimal::ZERO));
        assert_eq!(from_hex_units("0x", 18), Some(Decimal::ZERO));
    }

    #[test]
    fn asset_ids_and_price_keys() {
        let usdc = AssetMeta::token(Chain::Avalanche, "0xabc", 6);
        assert_eq!(usdc.id(), "avalanche:0xabc");
        assert_eq!(usdc.price_key, "avax:0xabc");
        let eth = AssetMeta::native(Chain::Base, "ETH", "Ether", 18, "ethereum");
        assert_eq!(eth.id(), "base:native");
        assert_eq!(eth.price_key, "coingecko:ethereum");
    }

    #[test]
    fn chain_round_trips() {
        for chain in
            Chain::EVM
                .into_iter()
                .chain([Chain::Solana, Chain::Bitcoin, Chain::Tron, Chain::Xrp])
        {
            assert_eq!(chain.as_str().parse::<Chain>().unwrap(), chain);
        }
    }
}
