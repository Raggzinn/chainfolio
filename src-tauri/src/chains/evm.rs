//! EVM chains: native balance over public JSON-RPC, ERC-20 balances from
//! Blockscout where its public API is reachable without a key, and from a
//! curated token list (batched `balanceOf`) everywhere else or as a fallback.
//! ENS names resolve through the on-chain registry.

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use sha3::{Digest, Keccak256};

use super::{
    AssetMeta, Chain, ChainAdapter, Holding,
    evm_tokens::{KnownToken, known_tokens},
    from_base_units, from_hex_units, rpc,
};
use crate::{
    address,
    error::{Error, Result},
};

struct Native {
    symbol: &'static str,
    name: &'static str,
    gecko_id: &'static str,
}

struct ChainConfig {
    rpcs: &'static [&'static str],
    /// Blockscout instance answering keyless API calls. The Arbitrum, Base and
    /// Polygon instances sit behind a bot challenge, so those use the list.
    blockscout: Option<&'static str>,
    native: Native,
}

const ETHER: Native = Native {
    symbol: "ETH",
    name: "Ether",
    gecko_id: "ethereum",
};

fn config(chain: Chain) -> ChainConfig {
    match chain {
        Chain::Ethereum => ChainConfig {
            rpcs: &[
                "https://ethereum-rpc.publicnode.com",
                "https://eth.drpc.org",
            ],
            blockscout: Some("https://eth.blockscout.com"),
            native: ETHER,
        },
        Chain::Arbitrum => ChainConfig {
            rpcs: &[
                "https://arbitrum-one-rpc.publicnode.com",
                "https://arb1.arbitrum.io/rpc",
            ],
            blockscout: None,
            native: ETHER,
        },
        Chain::Base => ChainConfig {
            rpcs: &[
                "https://base-rpc.publicnode.com",
                "https://mainnet.base.org",
            ],
            blockscout: None,
            native: ETHER,
        },
        Chain::Optimism => ChainConfig {
            rpcs: &[
                "https://optimism-rpc.publicnode.com",
                "https://mainnet.optimism.io",
            ],
            blockscout: Some("https://explorer.optimism.io"),
            native: ETHER,
        },
        Chain::Polygon => ChainConfig {
            rpcs: &[
                "https://polygon-bor-rpc.publicnode.com",
                "https://polygon-rpc.com",
            ],
            blockscout: None,
            native: Native {
                symbol: "POL",
                name: "Polygon",
                gecko_id: "polygon-ecosystem-token",
            },
        },
        Chain::Bsc => ChainConfig {
            rpcs: &[
                "https://bsc-rpc.publicnode.com",
                "https://bsc-dataseed.bnbchain.org",
            ],
            blockscout: None,
            native: Native {
                symbol: "BNB",
                name: "BNB",
                gecko_id: "binancecoin",
            },
        },
        Chain::Avalanche => ChainConfig {
            rpcs: &[
                "https://avalanche-c-chain-rpc.publicnode.com",
                "https://api.avax.network/ext/bc/C/rpc",
            ],
            blockscout: None,
            native: Native {
                symbol: "AVAX",
                name: "Avalanche",
                gecko_id: "avalanche-2",
            },
        },
        Chain::Solana | Chain::Bitcoin | Chain::Tron | Chain::Xrp => {
            unreachable!("{chain} is not an EVM chain")
        }
    }
}

/// Public JSON-RPC endpoints of an EVM chain, in fallback order.
pub fn rpcs(chain: Chain) -> &'static [&'static str] {
    config(chain).rpcs
}

/// Keyless Blockscout instance of an EVM chain, when one is reachable.
pub fn blockscout(chain: Chain) -> Option<&'static str> {
    config(chain).blockscout
}

/// ABI-encoded `balanceOf(address)` call data.
fn balance_of(owner: &str) -> String {
    format!(
        "0x70a08231{:0>64}",
        owner.trim_start_matches("0x").to_lowercase()
    )
}

pub struct EvmAdapter {
    chain: Chain,
    config: ChainConfig,
    http: reqwest::Client,
}

impl EvmAdapter {
    /// `decimals()` of every curated token, for the live tests.
    #[cfg(test)]
    pub async fn curated_decimals(&self) -> Result<Vec<(&'static str, u32, Option<u32>)>> {
        let tokens = known_tokens(self.chain);
        let calls = tokens
            .iter()
            .map(|t| json!([{ "to": t.contract, "data": "0x313ce567" }, "latest"]))
            .collect();
        let results = rpc::batch(&self.http, self.config.rpcs, "eth_call", calls).await?;
        Ok(tokens
            .iter()
            .zip(results)
            .map(|(t, r)| {
                let onchain = r
                    .and_then(|v| v.as_str().map(String::from))
                    .and_then(|hex| from_hex_units(&hex, 0))
                    .and_then(|d| u32::try_from(d.mantissa()).ok());
                (t.symbol, t.decimals, onchain)
            })
            .collect())
    }

    pub fn new(chain: Chain, http: reqwest::Client) -> Self {
        Self {
            chain,
            config: config(chain),
            http,
        }
    }

    async fn native_balance(&self, address: &str) -> Result<Option<Holding>> {
        let hex: String = rpc::call(
            &self.http,
            self.config.rpcs,
            "eth_getBalance",
            json!([address, "latest"]),
        )
        .await?;
        let amount = from_hex_units(&hex, 18)
            .ok_or_else(|| Error::provider(format!("bad balance {hex}")))?;
        let Native {
            symbol,
            name,
            gecko_id,
        } = self.config.native;
        Ok((!amount.is_zero()).then(|| Holding {
            asset: AssetMeta::native(self.chain, symbol, name, 18, gecko_id),
            amount,
            locked: None,
        }))
    }

    async fn blockscout_tokens(&self, base: &str, address: &str) -> Result<Vec<Holding>> {
        let url = format!("{base}/api/v2/addresses/{address}/token-balances");
        let response = self.http.get(url).send().await?;
        // Addresses Blockscout has never seen answer 404.
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(Vec::new());
        }
        let entries: Vec<TokenBalance> = response.error_for_status()?.json().await?;
        Ok(parse_token_balances(self.chain, entries))
    }

    async fn known_token_balances(&self, address: &str) -> Result<Vec<Holding>> {
        let tokens = known_tokens(self.chain);
        let data = balance_of(address);
        let calls = tokens
            .iter()
            .map(|token| json!([{ "to": token.contract, "data": data }, "latest"]))
            .collect();
        let results = rpc::batch(&self.http, self.config.rpcs, "eth_call", calls).await?;
        Ok(tokens
            .iter()
            .zip(results)
            .filter_map(|(token, result)| {
                let amount = from_hex_units(result?.as_str()?, token.decimals)?;
                (!amount.is_zero()).then(|| known_holding(self.chain, token, amount))
            })
            .collect())
    }
}

fn known_holding(chain: Chain, token: &KnownToken, amount: rust_decimal::Decimal) -> Holding {
    Holding {
        asset: AssetMeta::token(chain, token.contract, token.decimals)
            .with_names(Some(token.symbol.into()), Some(token.name.into())),
        amount,
        locked: None,
    }
}

#[async_trait]
impl ChainAdapter for EvmAdapter {
    fn chain(&self) -> Chain {
        self.chain
    }

    async fn balances(&self, address: &str) -> Result<Vec<Holding>> {
        let mut holdings: Vec<Holding> = self.native_balance(address).await?.into_iter().collect();
        let indexed = match self.config.blockscout {
            Some(base) => self.blockscout_tokens(base, address).await.ok(),
            None => None,
        };
        match indexed {
            Some(tokens) => holdings.extend(tokens),
            None => holdings.extend(self.known_token_balances(address).await?),
        }
        Ok(holdings)
    }
}

#[derive(Debug, Deserialize)]
struct TokenBalance {
    token: TokenInfo,
    value: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TokenInfo {
    // Newer Blockscout releases renamed `address` to `address_hash`.
    address: Option<String>,
    address_hash: Option<String>,
    decimals: Option<String>,
    name: Option<String>,
    symbol: Option<String>,
    #[serde(rename = "type")]
    kind: Option<String>,
    icon_url: Option<String>,
    /// USD price from Blockscout's market data; `null` for unlisted tokens.
    exchange_rate: Option<Value>,
}

fn parse_token_balances(chain: Chain, entries: Vec<TokenBalance>) -> Vec<Holding> {
    entries
        .into_iter()
        .filter_map(|entry| {
            let token = entry.token;
            // Unlisted tokens are overwhelmingly airdropped spam; skipping them
            // also keeps price lookups to a few requests for busy addresses.
            if token.kind.as_deref() != Some("ERC-20") || token.exchange_rate.is_none() {
                return None;
            }
            let contract = token.address_hash.or(token.address)?.to_lowercase();
            let decimals: u32 = token.decimals?.parse().ok()?;
            let amount = from_base_units(entry.value.as_deref()?, decimals)?;
            if amount.is_zero() {
                return None;
            }
            let mut asset =
                AssetMeta::token(chain, contract, decimals).with_names(token.symbol, token.name);
            asset.logo_url = token.icon_url.filter(|u| u.starts_with("https://"));
            Some(Holding {
                asset,
                amount,
                locked: None,
            })
        })
        .collect()
}

const ENS_REGISTRY: &str = "0x00000000000c2e074ec69a0dfb2997ba6c7d2e1e";

/// ENSIP-1 namehash of an already-normalized name.
pub fn namehash(name: &str) -> [u8; 32] {
    let mut node = [0u8; 32];
    for label in name.rsplit('.').filter(|l| !l.is_empty()) {
        let label_hash = Keccak256::digest(label.as_bytes());
        let mut hasher = Keccak256::new();
        hasher.update(node);
        hasher.update(label_hash);
        node.copy_from_slice(&hasher.finalize());
    }
    node
}

/// Resolves an ENS name to its checksummed `addr` record on Ethereum mainnet.
pub async fn resolve_ens(http: &reqwest::Client, name: &str) -> Result<String> {
    let not_found = || Error::NameNotFound(name.to_string());
    let node = hex::encode(namehash(name));
    let rpcs = config(Chain::Ethereum).rpcs;

    let call = |to: String, selector: &str| {
        let data = format!("0x{selector}{node}");
        async move {
            let word: String = rpc::call(
                http,
                rpcs,
                "eth_call",
                json!([{ "to": to, "data": data }, "latest"]),
            )
            .await?;
            Ok::<_, Error>(word_to_address(&word))
        }
    };

    // resolver(bytes32) on the registry, then addr(bytes32) on the resolver.
    let resolver = call(ENS_REGISTRY.into(), "0178b8bf")
        .await?
        .ok_or_else(not_found)?;
    let target = call(resolver, "3b3b57de").await?.ok_or_else(not_found)?;
    Ok(address::to_checksum(&target))
}

/// Last 20 bytes of a 32-byte ABI word as a lowercase address, `None` if zero.
fn word_to_address(word: &str) -> Option<String> {
    let hex = word.trim_start_matches("0x");
    if hex.len() < 40 {
        return None;
    }
    let tail = &hex[hex.len() - 40..];
    (!tail.bytes().all(|b| b == b'0')).then(|| format!("0x{}", tail.to_lowercase()))
}

#[cfg(test)]
mod tests {
    use rust_decimal::Decimal;

    use super::*;

    #[test]
    fn namehash_matches_ensip1() {
        assert_eq!(namehash(""), [0u8; 32]);
        assert_eq!(
            hex::encode(namehash("eth")),
            "93cdeb708b7545dc668eb9280176169d1c33cfd8ed6f04690a0bcc88a93fc4ae"
        );
        assert_eq!(
            hex::encode(namehash("foo.eth")),
            "de9b09fd7c5f901e23a3f19fecc54828e9c848539801e86591bd9801b019f84f"
        );
    }

    #[test]
    fn decodes_abi_address_words() {
        let word = "0x000000000000000000000000d8da6bf26964af9d7eed9e03e53415d37aa96045";
        assert_eq!(
            word_to_address(word).as_deref(),
            Some("0xd8da6bf26964af9d7eed9e03e53415d37aa96045")
        );
        assert_eq!(word_to_address(&format!("0x{}", "0".repeat(64))), None);
        assert_eq!(word_to_address("0x"), None);
    }

    #[test]
    fn parses_blockscout_token_balances() {
        let body = r#"[
          {"token": {"address_hash": "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48", "decimals": "6",
            "name": "USD Coin", "symbol": "USDC", "type": "ERC-20",
            "icon_url": "https://assets.coingecko.com/usdc.png", "exchange_rate": "0.9998"},
            "token_id": null, "value": "2500000"},
          {"token": {"address": "0x1111111111111111111111111111111111111111", "decimals": "18",
            "name": "Old", "symbol": "OLD", "type": "ERC-20", "icon_url": null, "exchange_rate": "2.1"},
            "value": "1000000000000000000"},
          {"token": {"address_hash": "0x4444444444444444444444444444444444444444", "decimals": "18",
            "name": "Visit claim-rewards.xyz", "symbol": "CLAIM", "type": "ERC-20", "exchange_rate": null},
            "value": "5000000000000000000000"},
          {"token": {"address_hash": "0x2222222222222222222222222222222222222222", "decimals": null,
            "name": "NFT", "symbol": "NFT", "type": "ERC-721"}, "value": "3"},
          {"token": {"address_hash": "0x3333333333333333333333333333333333333333", "decimals": "18",
            "name": "Zero", "symbol": "ZERO", "type": "ERC-20"}, "value": "0"}
        ]"#;
        let entries: Vec<TokenBalance> = serde_json::from_str(body).unwrap();
        let holdings = parse_token_balances(Chain::Ethereum, entries);

        assert_eq!(holdings.len(), 2);
        let usdc = &holdings[0];
        assert_eq!(usdc.amount, Decimal::new(25, 1));
        assert_eq!(
            usdc.asset.price_key,
            "ethereum:0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48"
        );
        assert_eq!(usdc.asset.symbol.as_deref(), Some("USDC"));
        assert!(usdc.asset.logo_url.is_some());
        assert_eq!(holdings[1].asset.symbol.as_deref(), Some("OLD"));
        assert_eq!(holdings[1].amount, Decimal::ONE);
    }
}
