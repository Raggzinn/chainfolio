//! Just enough Solidity ABI to read protocol contracts over `eth_call`:
//! selectors, static arguments and 32-byte result words.

use super::u256::U256;
use rust_decimal::Decimal;
use serde_json::json;
use sha3::{Digest, Keccak256};

use crate::{
    chains::{Chain, evm, from_base_units, rpc},
    error::Result,
};

/// Calls per JSON-RPC batch; free endpoints reject large batches.
const BATCH: usize = 40;

/// First four bytes of the Keccak hash of a function signature, as hex.
pub fn selector(signature: &str) -> String {
    hex::encode(&Keccak256::digest(signature.as_bytes())[..4])
}

/// A static ABI argument.
pub enum Arg<'a> {
    Address(&'a str),
    Uint(u64),
    Int(i64),
}

impl Arg<'_> {
    fn encode(&self) -> String {
        match self {
            Self::Address(a) => format!("{:0>64}", a.trim_start_matches("0x").to_lowercase()),
            Self::Uint(n) => format!("{n:064x}"),
            // Two's complement, sign-extended to 256 bits.
            Self::Int(n) if *n < 0 => format!("{}{:016x}", "f".repeat(48), *n as u64),
            Self::Int(n) => format!("{n:064x}"),
        }
    }
}

/// Call data for `signature` with static `args`.
pub fn call_data(signature: &str, args: &[Arg<'_>]) -> String {
    let mut data = format!("0x{}", selector(signature));
    for arg in args {
        data.push_str(&arg.encode());
    }
    data
}

/// One `eth_call` against `to`.
pub struct Call {
    pub to: String,
    pub data: String,
}

impl Call {
    pub fn new(to: impl Into<String>, signature: &str, args: &[Arg<'_>]) -> Self {
        Self {
            to: to.into(),
            data: call_data(signature, args),
        }
    }
}

/// Runs `calls` as chunked JSON-RPC batches on `chain`. Each slot is the raw
/// return data, or `None` when that call reverted or the node skipped it.
pub async fn eth_calls(
    http: &reqwest::Client,
    chain: Chain,
    calls: Vec<Call>,
) -> Result<Vec<Option<Words>>> {
    let rpcs = evm::rpcs(chain);
    let params: Vec<_> = calls
        .iter()
        .map(|c| json!([{ "to": c.to, "data": c.data }, "latest"]))
        .collect();
    let mut out = Vec::with_capacity(params.len());
    // Sequential on purpose: free endpoints throttle parallel batches.
    for chunk in params.chunks(BATCH) {
        let results = rpc::batch(http, rpcs, "eth_call", chunk.to_vec()).await?;
        out.extend(
            results
                .into_iter()
                .map(|v| v.and_then(|v| v.as_str().and_then(Words::parse))),
        );
    }
    Ok(out)
}

/// Single `eth_call`, failing when the call reverts.
pub async fn eth_call(http: &reqwest::Client, chain: Chain, call: Call) -> Result<Words> {
    let hex: String = rpc::call(
        http,
        evm::rpcs(chain),
        "eth_call",
        json!([{ "to": call.to, "data": call.data }, "latest"]),
    )
    .await?;
    Words::parse(&hex).ok_or_else(|| crate::error::Error::provider("malformed eth_call result"))
}

/// ABI return data split into 32-byte words.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Words(Vec<[u8; 32]>);

impl Words {
    pub fn parse(hex: &str) -> Option<Self> {
        let bytes = hex::decode(hex.trim_start_matches("0x")).ok()?;
        if bytes.len() % 32 != 0 {
            return None;
        }
        Some(Self(
            bytes
                .chunks_exact(32)
                .map(|c| c.try_into().expect("32-byte chunk"))
                .collect(),
        ))
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn uint(&self, i: usize) -> Option<U256> {
        self.0.get(i).map(|w| U256::from_be_bytes(*w))
    }

    /// A signed word that fits in 64 bits (ticks, int24/int56 values).
    pub fn int(&self, i: usize) -> Option<i64> {
        let word = self.0.get(i)?;
        Some(i64::from_be_bytes(word[24..].try_into().ok()?))
    }

    /// Lowercase `0x` address in word `i`.
    pub fn address(&self, i: usize) -> Option<String> {
        self.0
            .get(i)
            .map(|w| format!("0x{}", hex::encode(&w[12..])))
    }

    /// Word `i` as a token amount with `decimals`.
    pub fn amount(&self, i: usize, decimals: u32) -> Option<Decimal> {
        to_decimal(self.uint(i)?, decimals)
    }

    /// `string` return value, also accepting the `bytes32` that old tokens
    /// (MKR, SAI) return from `symbol()`.
    pub fn string(&self) -> Option<String> {
        let raw: Vec<u8> = if self.0.len() == 1 {
            self.0[0].iter().copied().take_while(|b| *b != 0).collect()
        } else {
            let offset = usize::try_from(self.uint(0)?).ok()? / 32;
            let len = usize::try_from(self.uint(offset)?).ok()?;
            let bytes: Vec<u8> = self
                .0
                .get(offset + 1..)?
                .iter()
                .flatten()
                .copied()
                .collect();
            bytes.get(..len)?.to_vec()
        };
        let s = String::from_utf8(raw).ok()?;
        let s = s.trim();
        (!s.is_empty() && s.len() <= 32).then(|| s.to_string())
    }

    /// Dynamic `address[]` return value.
    pub fn addresses(&self) -> Option<Vec<String>> {
        let offset = usize::try_from(self.uint(0)?).ok()? / 32;
        let len = usize::try_from(self.uint(offset)?).ok()?;
        (offset + 1..offset + 1 + len)
            .map(|i| self.address(i))
            .collect()
    }
}

/// An integer amount in base units as a decimal with `decimals`.
pub fn to_decimal(value: U256, decimals: u32) -> Option<Decimal> {
    from_base_units(&value.to_string(), decimals)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computes_selectors() {
        assert_eq!(selector("balanceOf(address)"), "70a08231");
        assert_eq!(selector("getUserAccountData(address)"), "bf92857c");
        assert_eq!(selector("baseToken()"), "c55dae63");
        assert_eq!(selector("allowance(address,address)"), "dd62ed3e");
    }

    #[test]
    fn encodes_static_arguments() {
        let data = call_data("ticks(int24)", &[Arg::Int(-887_220)]);
        assert_eq!(&data[10..58], "f".repeat(48));
        assert!(data.ends_with("fffffffffff2764c"));
        let data = call_data(
            "tokenOfOwnerByIndex(address,uint256)",
            &[
                Arg::Address("0xD8dA6BF26964aF9D7eEd9e03E53415D37aA96045"),
                Arg::Uint(3),
            ],
        );
        assert_eq!(data.len(), 2 + 8 + 128);
        assert!(data.contains("d8da6bf26964af9d7eed9e03e53415d37aa96045"));
        assert!(data.ends_with("03"));
    }

    #[test]
    fn decodes_words() {
        let words = Words::parse(&format!(
            "0x{:0>64}{}{:064x}",
            "c02aaa39b223fe8d0a0e5c4f27ead9083c756cc2",
            "f".repeat(58) + "f2764c",
            1_500_000u64,
        ))
        .unwrap();
        assert_eq!(words.len(), 3);
        assert_eq!(
            words.address(0).unwrap(),
            "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2"
        );
        assert_eq!(words.int(1), Some(-887_220));
        assert_eq!(words.amount(2, 6), Some(Decimal::new(15, 1)));
        assert!(Words::parse("0x1234").is_none());
    }

    #[test]
    fn decodes_strings_and_bytes32() {
        let usdc = format!("0x{:064x}{:064x}{:0<64}", 32, 4, hex::encode("USDC"));
        assert_eq!(
            Words::parse(&usdc).unwrap().string().as_deref(),
            Some("USDC")
        );
        let mkr = format!("0x{:0<64}", hex::encode("MKR"));
        assert_eq!(Words::parse(&mkr).unwrap().string().as_deref(), Some("MKR"));
    }

    #[test]
    fn decodes_address_arrays() {
        let data = format!("0x{:064x}{:064x}{:0>64}{:0>64}", 32, 2, "aa", "bb");
        let list = Words::parse(&data).unwrap().addresses().unwrap();
        assert_eq!(list.len(), 2);
        assert!(list[1].ends_with("bb"));
    }
}
