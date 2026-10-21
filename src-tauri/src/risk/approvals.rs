//! Open ERC-20 approvals: `Approval` logs of the owner from a keyless
//! Blockscout instance, confirmed with `allowance()` over RPC because later
//! transfers spend allowances without emitting new logs. Keyless Blockscout
//! allows about ten requests per ten minutes, so a scan spends as few as it can.

use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    time::Duration,
};

use serde::Deserialize;

use crate::{
    chains::{Chain, evm},
    defi::{
        abi::{Arg, Call, eth_calls},
        u256::U256,
    },
    error::{Error, Result},
    prices::{PriceProvider, defillama::DefiLlama},
};

/// keccak256("Approval(address,address,uint256)")
const APPROVAL_TOPIC: &str = "0x8c5be1e5ebec7d5bd14f71427d1e84f3dd0314c0f7b2291e5b200ac8c7c3b925";
/// Blockscout returns at most 1000 logs per page.
const PAGE: usize = 1000;
const MAX_PAGES: usize = 3;
/// Most recently approved pairs re-checked on-chain.
const MAX_PAIRS: usize = 300;

/// Well-known spenders, named without spending an API request.
const KNOWN_SPENDERS: &[(&str, &str)] = &[
    (
        "0x000000000022d473030f116ddee9f6b43ac78ba3",
        "Uniswap Permit2",
    ),
    (
        "0x3fc91a3afd70395cd496c647d5a6cc9d4b2b7fad",
        "Uniswap Universal Router",
    ),
    (
        "0x68b3465833fb72a70ecdf485e0e4c7bd8665fc45",
        "Uniswap SwapRouter02",
    ),
    (
        "0xe592427a0aece92de3edee1f18e0157c05861564",
        "Uniswap V3 Router",
    ),
    (
        "0x7a250d5630b4cf539739df2c5dacb4c659f2488d",
        "Uniswap V2 Router",
    ),
    (
        "0x1111111254eeb25477b68fb85ed929f73a960582",
        "1inch Router v5",
    ),
    (
        "0x111111125421ca6dc452d289314280a0f8842a65",
        "1inch Router v6",
    ),
    (
        "0xdef1c0ded9bec7f1a1670819833240f027b25eff",
        "0x Exchange Proxy",
    ),
    (
        "0x1e0049783f008a0085193e00003d00cd54003c71",
        "OpenSea Seaport",
    ),
    ("0x87870bca3f3fd6335c3f4ce8392d69350b4fa4e2", "Aave v3 Pool"),
    ("0x794a61358d6845594f94dc1db02a252b5b4814ad", "Aave v3 Pool"),
    (
        "0x881d40237659c251811cec9c364ef91dc08d300c",
        "MetaMask Swaps",
    ),
    ("0xdef171fe48cf0115b1d80b88dc8eab59176fee57", "ParaSwap v5"),
    ("0x6a000f20005980200259b80c5102003040001068", "ParaSwap v6"),
    ("0xc92e8bdf79f0507f65a392b0ab4667716bfe0110", "CoW Protocol"),
    (
        "0xba12222222228d8ba445958a75a0704d566bf2c8",
        "Balancer Vault",
    ),
    (
        "0xd9e1ce17f2641f24ae83637ab66a2cca9c378b9f",
        "SushiSwap Router",
    ),
    (
        "0x00000000000000adc04c56bf30ac9d3c0aaf14dc",
        "OpenSea Seaport",
    ),
];

#[derive(Debug, Deserialize)]
struct LogsResponse {
    result: serde_json::Value,
    message: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Log {
    address: String,
    topics: Vec<Option<String>>,
    #[serde(rename = "blockNumber")]
    block_number: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OpenApproval {
    pub token: String,
    pub spender: String,
    /// Raw allowance in token base units.
    pub amount: U256,
    pub unlimited: bool,
    pub token_symbol: Option<String>,
    pub token_decimals: Option<u32>,
    pub spender_name: Option<String>,
}

/// Allowances at or above `type(uint96).max` are "infinite" in practice
/// (UNI and COMP cap allowances at uint96; most tokens use uint256).
pub fn is_unlimited(raw: U256) -> bool {
    raw >= (U256::from(1u8) << 96) - U256::from(1u8)
}

fn topic_address(topic: &str) -> Option<String> {
    let hex = topic.trim_start_matches("0x");
    (hex.len() == 64).then(|| format!("0x{}", hex[24..].to_lowercase()))
}

/// Distinct (token, spender) pairs, most recent approval first. ERC-721
/// approvals share the event signature but index the token id as a 4th topic.
pub fn approval_pairs(logs: &[Log]) -> Vec<(String, String)> {
    let mut seen = HashSet::new();
    let mut pairs = Vec::new();
    let mut ordered: Vec<&Log> = logs.iter().collect();
    ordered.sort_by_key(|l| std::cmp::Reverse(block(&l.block_number)));
    for log in ordered {
        let erc20 = log.topics.len() == 3 || log.topics.get(3).is_some_and(Option::is_none);
        let spender = log
            .topics
            .get(2)
            .and_then(|t| t.as_deref())
            .and_then(topic_address);
        if let (true, Some(spender)) = (erc20, spender) {
            let pair = (log.address.to_lowercase(), spender);
            if seen.insert(pair.clone()) {
                pairs.push(pair);
            }
        }
    }
    pairs
}

fn block(hex: &str) -> u64 {
    u64::from_str_radix(hex.trim_start_matches("0x"), 16).unwrap_or(0)
}

async fn logs(http: &reqwest::Client, base: &str, owner: &str) -> Result<Vec<Log>> {
    let owner_topic = format!("0x{:0>64}", owner.trim_start_matches("0x").to_lowercase());
    let mut all = Vec::new();
    let mut from = 0u64;
    for page_number in 0..MAX_PAGES {
        if page_number > 0 {
            // The keyless API throttles quick successive requests.
            tokio::time::sleep(Duration::from_millis(600)).await;
        }
        let page = match page(http, base, from, &owner_topic).await {
            Ok(page) => page,
            // Keep what was read so far; the next scan tries again.
            Err(_) if !all.is_empty() => break,
            Err(err) => return Err(err),
        };
        let full = page.len() >= PAGE;
        let last = page
            .iter()
            .map(|l| block(&l.block_number))
            .max()
            .unwrap_or(from);
        all.extend(page);
        if !full || last <= from {
            break;
        }
        // Logs of the boundary block may repeat; pairs are deduplicated.
        from = last;
    }
    Ok(all)
}

async fn page(
    http: &reqwest::Client,
    base: &str,
    from: u64,
    owner_topic: &str,
) -> Result<Vec<Log>> {
    let response = http
        .get(format!(
            "{base}/api?module=logs&action=getLogs&fromBlock={from}&toBlock=latest\
                 &topic0={APPROVAL_TOPIC}&topic1={owner_topic}&topic0_1_opr=and"
        ))
        .send()
        .await?;
    if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
        return Err(Error::provider(
            "Blockscout rate limit reached; approvals refresh on a later scan",
        ));
    }
    let response: LogsResponse = response.error_for_status()?.json().await?;
    match response.result {
        serde_json::Value::Array(_) => serde_json::from_value(response.result)
            .map_err(|e| Error::provider(format!("blockscout logs: {e}"))),
        // "No logs found" comes back as an empty result or a null one.
        serde_json::Value::Null => Ok(Vec::new()),
        other => Err(Error::provider(format!(
            "blockscout logs: {}",
            response.message.unwrap_or_else(|| other.to_string())
        ))),
    }
}

fn spender_name(spender: &str) -> Option<String> {
    KNOWN_SPENDERS
        .iter()
        .find(|(address, _)| *address == spender)
        .map(|(_, name)| (*name).to_string())
}

/// Every non-zero ERC-20 allowance `owner` has granted on `chain`. Chains
/// without a keyless Blockscout return an error the caller can report.
pub async fn scan(http: &reqwest::Client, chain: Chain, owner: &str) -> Result<Vec<OpenApproval>> {
    let base = evm::blockscout(chain)
        .ok_or_else(|| Error::provider(format!("no keyless log index for {chain}")))?;
    let mut pairs = approval_pairs(&logs(http, base, owner).await?);
    // Airdropped spam tokens emit fake `Approval` events by the thousand; an
    // allowance on a token without a market price puts nothing at risk.
    let keys: Vec<String> = pairs
        .iter()
        .map(|(token, _)| format!("{}:{token}", chain.llama_prefix()))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let quotes = DefiLlama::new(http.clone()).quotes(&keys).await?;
    let priced: HashSet<String> = quotes.keys().map(|k| k.to_lowercase()).collect();
    pairs.retain(|(token, _)| priced.contains(&format!("{}:{token}", chain.llama_prefix())));
    pairs.truncate(MAX_PAIRS);
    let allowances = eth_calls(
        http,
        chain,
        pairs
            .iter()
            .map(|(token, spender)| {
                Call::new(
                    token.as_str(),
                    "allowance(address,address)",
                    &[Arg::Address(owner), Arg::Address(spender)],
                )
            })
            .collect(),
    )
    .await?;
    let open: Vec<((String, String), U256)> = pairs
        .into_iter()
        .zip(allowances)
        .filter_map(|(pair, words)| {
            let amount = words?.uint(0)?;
            (!amount.is_zero()).then_some((pair, amount))
        })
        .collect();
    if open.is_empty() {
        return Ok(Vec::new());
    }

    let tokens: Vec<String> = open
        .iter()
        .map(|((t, _), _)| t.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let meta = eth_calls(
        http,
        chain,
        tokens
            .iter()
            .flat_map(|t| {
                [
                    Call::new(t.as_str(), "symbol()", &[]),
                    Call::new(t.as_str(), "decimals()", &[]),
                ]
            })
            .collect(),
    )
    .await?;
    let token_meta: BTreeMap<&String, (Option<String>, Option<u32>)> = tokens
        .iter()
        .zip(meta.chunks(2))
        .map(|(token, m)| {
            let symbol = m[0].as_ref().and_then(|w| w.string());
            let decimals = m[1]
                .as_ref()
                .and_then(|w| w.uint(0))
                .and_then(|d| u32::try_from(d).ok())
                .filter(|d| *d <= 36);
            (token, (symbol, decimals))
        })
        .collect();

    Ok(open
        .into_iter()
        .map(|((token, spender), amount)| {
            let (symbol, decimals) = token_meta.get(&token).cloned().unwrap_or_default();
            OpenApproval {
                spender_name: spender_name(&spender),
                unlimited: is_unlimited(amount),
                token_symbol: symbol,
                token_decimals: decimals,
                token,
                spender,
                amount,
            }
        })
        .collect())
}

/// revoke.cash page listing the owner's approvals on one chain.
pub fn revoke_url(owner: &str, chain: Chain) -> Option<String> {
    let id = super::goplus::chain_id(chain)?;
    Some(format!("https://revoke.cash/address/{owner}?chainId={id}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn log(token: &str, spender: &str, block: u64, nft: bool) -> Log {
        let mut topics = vec![
            Some(APPROVAL_TOPIC.to_string()),
            Some(format!(
                "0x{:0>64}",
                "d8da6bf26964af9d7eed9e03e53415d37aa96045"
            )),
            Some(format!("0x{:0>64}", spender)),
        ];
        topics.push(if nft {
            Some(format!("0x{:064x}", 42))
        } else {
            None
        });
        Log {
            address: token.into(),
            topics,
            block_number: format!("0x{block:x}"),
        }
    }

    #[test]
    fn collects_distinct_pairs_newest_first() {
        let logs = vec![
            log("0xTokenA", "aaaa", 10, false),
            log("0xtokenb", "bbbb", 30, false),
            log("0xtokena", "aaaa", 20, false),
            log("0xnft", "cccc", 40, true),
        ];
        let pairs = approval_pairs(&logs);
        assert_eq!(pairs.len(), 2);
        assert_eq!(pairs[0].0, "0xtokenb");
        assert_eq!(pairs[1], ("0xtokena".into(), format!("0x{:0>40}", "aaaa")));
    }

    #[test]
    fn detects_unlimited_allowances() {
        assert!(is_unlimited(U256::MAX));
        assert!(is_unlimited((U256::from(1u8) << 96) - U256::from(1u8)));
        assert!(is_unlimited((U256::from(1u8) << 160) - U256::from(1u8)));
        assert!(!is_unlimited(U256::from(1_000_000_000_000_000_000_000u128)));
    }

    #[test]
    fn builds_revoke_links() {
        assert_eq!(
            revoke_url("0xabc", Chain::Optimism).as_deref(),
            Some("https://revoke.cash/address/0xabc?chainId=10")
        );
        assert!(revoke_url("x", Chain::Solana).is_none());
    }
}
