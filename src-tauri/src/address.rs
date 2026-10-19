//! Offline detection and validation of what the user pastes into "Add wallet".
//! Names (ENS/SNS) are only recognized here; resolving them needs the network.

use std::str::FromStr;

use serde::Serialize;
use sha3::{Digest, Keccak256};

use crate::{
    chains::{Chain, Family, bitcoin},
    error::{Error, Result},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum InputKind {
    Address,
    Ens,
    Sns,
    Xpub,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Detection {
    pub family: Family,
    pub kind: InputKind,
    /// Canonical form: checksummed EVM address, lowercase name, bech32 in
    /// lowercase, or the key/address exactly as given.
    pub normalized: String,
    /// Chains that will be scanned for this input.
    pub chains: Vec<Chain>,
}

impl Detection {
    fn new(family: Family, kind: InputKind, normalized: String) -> Self {
        Self {
            family,
            kind,
            normalized,
            chains: family.chains().to_vec(),
        }
    }
}

pub fn detect(input: &str) -> Result<Detection> {
    let input = input.trim();
    if input.is_empty() {
        return Err(Error::InvalidAddress);
    }

    if let Some(hex) = input
        .strip_prefix("0x")
        .or_else(|| input.strip_prefix("0X"))
    {
        return evm_address(hex).map(|a| Detection::new(Family::Evm, InputKind::Address, a));
    }

    let lower = input.to_lowercase();
    if is_name(&lower, ".eth") {
        return Ok(Detection::new(Family::Evm, InputKind::Ens, lower));
    }
    if is_name(&lower, ".sol") {
        return Ok(Detection::new(Family::Solana, InputKind::Sns, lower));
    }

    if bitcoin::is_extended_key(input) {
        bitcoin::parse_extended_key(input)?;
        return Ok(Detection::new(
            Family::Bitcoin,
            InputKind::Xpub,
            input.into(),
        ));
    }

    if let Ok(address) = ::bitcoin::Address::from_str(input) {
        let address = address
            .require_network(::bitcoin::Network::Bitcoin)
            .map_err(|_| Error::InvalidAddress)?;
        return Ok(Detection::new(
            Family::Bitcoin,
            InputKind::Address,
            address.to_string(),
        ));
    }

    if is_solana_address(input) {
        return Ok(Detection::new(
            Family::Solana,
            InputKind::Address,
            input.into(),
        ));
    }

    Err(Error::InvalidAddress)
}

/// `label(.label)*.tld` with DNS-ish labels. Full ENSIP-15 normalization
/// (emoji, confusables) is out of scope; lowercase ASCII and unicode letters
/// cover the vast majority of names.
fn is_name(lower: &str, tld: &str) -> bool {
    let Some(rest) = lower.strip_suffix(tld) else {
        return false;
    };
    !rest.is_empty()
        && rest.split('.').all(|label| {
            !label.is_empty()
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        })
}

/// Validates a 20-byte hex address. All-lowercase or all-uppercase inputs are
/// accepted as-is; mixed case must match the EIP-55 checksum.
fn evm_address(hex: &str) -> Result<String> {
    if hex.len() != 40 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(Error::InvalidAddress);
    }
    let checksummed = to_checksum(hex);
    let has_lower = hex.bytes().any(|b| b.is_ascii_lowercase());
    let has_upper = hex.bytes().any(|b| b.is_ascii_uppercase());
    if has_lower && has_upper && checksummed[2..] != *hex {
        return Err(Error::BadChecksum);
    }
    Ok(checksummed)
}

/// EIP-55 mixed-case checksum encoding of an address (with or without `0x`).
pub fn to_checksum(address: &str) -> String {
    let lower = address.trim_start_matches("0x").to_lowercase();
    let hash = Keccak256::digest(lower.as_bytes());
    let encoded: String = lower
        .chars()
        .enumerate()
        .map(|(i, c)| {
            let nibble = (hash[i / 2] >> if i % 2 == 0 { 4 } else { 0 }) & 0x0f;
            if c.is_ascii_alphabetic() && nibble >= 8 {
                c.to_ascii_uppercase()
            } else {
                c
            }
        })
        .collect();
    format!("0x{encoded}")
}

/// A Solana address is any base58 string that decodes to 32 bytes.
pub fn is_solana_address(input: &str) -> bool {
    (32..=44).contains(&input.len())
        && bs58::decode(input)
            .into_vec()
            .is_ok_and(|bytes| bytes.len() == 32)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kind(input: &str) -> (Family, InputKind) {
        let d = detect(input).unwrap();
        (d.family, d.kind)
    }

    #[test]
    fn detects_evm_addresses() {
        let d = detect("  0xd8da6bf26964af9d7eed9e03e53415d37aa96045 ").unwrap();
        assert_eq!(d.family, Family::Evm);
        assert_eq!(d.normalized, "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045");
        assert_eq!(d.chains.len(), 7);
        assert!(detect("0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045").is_ok());
    }

    #[test]
    fn rejects_bad_evm_checksums_and_lengths() {
        assert!(matches!(
            detect("0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96046"),
            Err(Error::BadChecksum)
        ));
        assert!(matches!(
            detect("0xD8DA6BF26964AF9D7EED9E03E53415D37AA9604"),
            Err(Error::InvalidAddress)
        ));
    }

    #[test]
    fn eip55_reference_vectors() {
        for addr in [
            "0x5aAeb6053F3E94C9b9A09f33669435E7Ef1BeAed",
            "0xfB6916095ca1df60bB79Ce92cE3Ea74c37c5d359",
            "0xdbF03B407c01E7cD3CBea99509d93f8DDDC8C6FB",
            "0xD1220A0cf47c7B9Be7A2E6BA89F429762e7b9aDb",
        ] {
            assert_eq!(to_checksum(&addr.to_lowercase()), addr);
        }
    }

    #[test]
    fn detects_names() {
        assert_eq!(kind("Vitalik.ETH"), (Family::Evm, InputKind::Ens));
        assert_eq!(detect("Vitalik.ETH").unwrap().normalized, "vitalik.eth");
        assert_eq!(kind("pay.vitalik.eth"), (Family::Evm, InputKind::Ens));
        assert_eq!(kind("bonfida.sol"), (Family::Solana, InputKind::Sns));
        assert!(detect(".eth").is_err());
        assert!(detect("bad..eth").is_err());
        assert!(detect("-bad.eth").is_err());
    }

    #[test]
    fn detects_bitcoin_addresses() {
        for addr in [
            "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa",
            "3J98t1WpEZ73CNmQviecrnyiWrnqRhWNLy",
            "bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq",
            "bc1p5d7rjq7g6rdk2yhzks9smlaqtedr4dekq08ge8ztwac72sfr9rusxg3297",
        ] {
            assert_eq!(kind(addr), (Family::Bitcoin, InputKind::Address), "{addr}");
        }
        // Bech32 is case-insensitive; the canonical form is lowercase.
        assert_eq!(
            detect("BC1QAR0SRRR7XFKVY5L643LYDNW9RE59GTZZWF5MDQ")
                .unwrap()
                .normalized,
            "bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq"
        );
        // Testnet addresses are not supported.
        assert!(detect("tb1qw508d6qejxtdg4y5r3zarvary0c5xw7kxpjzsx").is_err());
    }

    #[test]
    fn detects_extended_keys() {
        let zpub = "zpub6rFR7y4Q2AijBEqTUquhVz398htDFrtymD9xYYfG1m4wAcvPhXNfE3EfH1r1ADqtfSdVCToUG868RvUUkgDKf31mGDtKsAYz2oz2AGutZYs";
        assert_eq!(kind(zpub), (Family::Bitcoin, InputKind::Xpub));
        assert!(detect(&zpub[..zpub.len() - 1]).is_err());
    }

    #[test]
    fn detects_solana_addresses() {
        assert_eq!(
            kind("9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM"),
            (Family::Solana, InputKind::Address)
        );
        assert!(detect("9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYt0WWM").is_err());
    }

    #[test]
    fn rejects_garbage() {
        for input in ["", "hello", "0x", "not an address at all"] {
            assert!(detect(input).is_err(), "{input}");
        }
    }
}
