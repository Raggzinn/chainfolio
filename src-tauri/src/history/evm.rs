//! EVM history from Blockscout (keyless on Ethereum and Optimism), Routescan
//! (keyless, Etherscan-compatible, Avalanche) or Etherscan V2 with the user's
//! own key for every other chain. All three report native, ERC-20 and internal
//! transfers as separate streams that are merged by hash.

use async_trait::async_trait;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{ApiKeys, HistoryAdapter, Page, RawTx, merge_by_hash};
use crate::{
    chains::{AssetMeta, Chain, evm::native_asset, from_base_units},
    error::{Error, Result},
};

#[derive(Clone, Debug, PartialEq)]
pub enum Source {
    Blockscout(&'static str),
    /// Etherscan-compatible API; `key` is appended when present.
    Etherscan {
        base: String,
        key: Option<String>,
    },
}

fn chain_id(chain: Chain) -> u32 {
    match chain {
        Chain::Ethereum => 1,
        Chain::Optimism => 10,
        Chain::Bsc => 56,
        Chain::Polygon => 137,
        Chain::Base => 8453,
        Chain::Arbitrum => 42161,
        Chain::Avalanche => 43114,
        _ => 0,
    }
}

/// Where `chain`'s history comes from, or `None` when it needs a key the user
/// hasn't provided.
pub fn source(chain: Chain, keys: &ApiKeys) -> Option<Source> {
    let etherscan = |key: &String| Source::Etherscan {
        base: format!(
            "https://api.etherscan.io/v2/api?chainid={}",
            chain_id(chain)
        ),
        key: Some(key.clone()),
    };
    match chain {
        Chain::Ethereum => Some(
            keys.etherscan
                .as_ref()
                .map_or(Source::Blockscout("https://eth.blockscout.com"), etherscan),
        ),
        Chain::Optimism => Some(Source::Blockscout("https://explorer.optimism.io")),
        Chain::Avalanche => Some(Source::Etherscan {
            base: "https://api.routescan.io/v2/network/mainnet/evm/43114/etherscan/api?".into(),
            key: None,
        }),
        _ => keys.etherscan.as_ref().map(etherscan),
    }
}

pub struct EvmHistory {
    chain: Chain,
    source: Source,
    http: reqwest::Client,
}

impl EvmHistory {
    pub fn new(chain: Chain, source: Source, http: reqwest::Client) -> Self {
        Self {
            chain,
            source,
            http,
        }
    }
}

#[async_trait]
impl HistoryAdapter for EvmHistory {
    fn chain(&self) -> Chain {
        self.chain
    }

    async fn page(&self, owners: &[String], cursor: Option<&str>) -> Result<Page> {
        let owner = owners
            .first()
            .ok_or_else(|| Error::provider("no address"))?;
        match &self.source {
            Source::Blockscout(base) => self.blockscout(base, owner, cursor).await,
            Source::Etherscan { base, key } => {
                self.etherscan(base, key.as_deref(), owner, cursor).await
            }
        }
    }
}

// ---------------------------------------------------------------- Blockscout

/// Next-page parameters of the three Blockscout streams; a missing entry means
/// that stream is exhausted.
#[derive(Debug, Default, Serialize, Deserialize)]
struct BlockscoutCursor {
    tx: Option<Value>,
    tt: Option<Value>,
    it: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct Listing<T> {
    items: Vec<T>,
    next_page_params: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct BsAddress {
    hash: String,
}

#[derive(Debug, Deserialize)]
struct BsFee {
    value: Option<String>,
}

#[derive(Debug, Deserialize)]
struct BsTx {
    hash: String,
    timestamp: Option<String>,
    value: Option<String>,
    from: Option<BsAddress>,
    to: Option<BsAddress>,
    fee: Option<BsFee>,
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct BsToken {
    address: Option<String>,
    address_hash: Option<String>,
    symbol: Option<String>,
    name: Option<String>,
    decimals: Option<String>,
    #[serde(rename = "type")]
    kind: Option<String>,
}

#[derive(Debug, Deserialize)]
struct BsTotal {
    value: Option<String>,
}

#[derive(Debug, Deserialize)]
struct BsTokenTransfer {
    transaction_hash: String,
    timestamp: Option<String>,
    from: Option<BsAddress>,
    to: Option<BsAddress>,
    token: BsToken,
    total: Option<BsTotal>,
}

#[derive(Debug, Deserialize)]
struct BsInternal {
    transaction_hash: String,
    timestamp: Option<String>,
    from: Option<BsAddress>,
    to: Option<BsAddress>,
    value: Option<String>,
    success: Option<bool>,
}

fn iso_ms(ts: Option<&str>) -> i64 {
    ts.and_then(chrono_like_parse).unwrap_or(0)
}

/// Minimal RFC 3339 parser (Blockscout timestamps are always UTC, `Z` suffix,
/// optional fraction) so we don't pull in a date crate for one format.
fn chrono_like_parse(ts: &str) -> Option<i64> {
    let ts = ts.trim_end_matches('Z');
    let (date, time) = ts.split_once('T')?;
    let mut d = date.split('-').map(|p| p.parse::<i64>());
    let (y, m, day) = (d.next()?.ok()?, d.next()?.ok()?, d.next()?.ok()?);
    let time = time.split('.').next()?;
    let mut t = time.split(':').map(|p| p.parse::<i64>());
    let (h, min, s) = (t.next()?.ok()?, t.next()?.ok()?, t.next()?.ok()?);
    // Days from civil (Howard Hinnant's algorithm).
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    Some(((days * 86_400) + h * 3600 + min * 60 + s) * 1000)
}

fn same(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
}

/// Signed amount for `owner` of a transfer `from` → `to`; zero if it isn't involved.
fn direction(
    owner: &str,
    from: Option<&str>,
    to: Option<&str>,
    amount: Decimal,
) -> (Decimal, Option<String>) {
    let from_owner = from.is_some_and(|f| same(f, owner));
    let to_owner = to.is_some_and(|t| same(t, owner));
    match (from_owner, to_owner) {
        (true, false) => (-amount, to.map(String::from)),
        (false, true) => (amount, from.map(String::from)),
        _ => (Decimal::ZERO, None),
    }
}

impl EvmHistory {
    async fn blockscout(&self, base: &str, owner: &str, cursor: Option<&str>) -> Result<Page> {
        let state: BlockscoutCursor = match cursor {
            Some(c) => serde_json::from_str(c).map_err(|e| Error::provider(e.to_string()))?,
            None => BlockscoutCursor {
                tx: Some(Value::Null),
                tt: Some(Value::Null),
                it: Some(Value::Null),
            },
        };
        let native = native_asset(self.chain);
        let mut txs = Vec::new();
        let mut next = BlockscoutCursor::default();

        if let Some(params) = &state.tx {
            let page: Listing<BsTx> = self
                .bs_get(
                    &format!("{base}/api/v2/addresses/{owner}/transactions"),
                    params,
                )
                .await?;
            next.tx = page.next_page_params;
            for item in page.items {
                let mut tx = RawTx::new(&item.hash, iso_ms(item.timestamp.as_deref()));
                tx.failed = item.status.as_deref() == Some("error");
                let from = item.from.as_ref().map(|a| a.hash.as_str());
                let value = item
                    .value
                    .as_deref()
                    .and_then(|v| from_base_units(v, 18))
                    .unwrap_or_default();
                if !tx.failed {
                    let (amount, cp) = direction(
                        owner,
                        from,
                        item.to.as_ref().map(|a| a.hash.as_str()),
                        value,
                    );
                    tx.add(native.clone(), amount, cp);
                }
                if from.is_some_and(|f| same(f, owner)) {
                    let fee = item
                        .fee
                        .and_then(|f| f.value)
                        .and_then(|v| from_base_units(&v, 18))
                        .unwrap_or_default();
                    if !fee.is_zero() {
                        tx.fee = Some((native.clone(), fee));
                    }
                }
                txs.push(tx);
            }
        }

        if let Some(params) = &state.tt {
            let page: Listing<BsTokenTransfer> = self
                .bs_get(
                    &format!("{base}/api/v2/addresses/{owner}/token-transfers?type=ERC-20"),
                    params,
                )
                .await?;
            next.tt = page.next_page_params;
            for item in page.items {
                if item.token.kind.as_deref().is_some_and(|k| k != "ERC-20") {
                    continue;
                }
                let Some(contract) = item.token.address_hash.or(item.token.address) else {
                    continue;
                };
                let decimals = item
                    .token
                    .decimals
                    .as_deref()
                    .and_then(|d| d.parse().ok())
                    .unwrap_or(18);
                let Some(amount) = item
                    .total
                    .and_then(|t| t.value)
                    .and_then(|v| from_base_units(&v, decimals))
                else {
                    continue;
                };
                let asset = AssetMeta::token(self.chain, contract.to_lowercase(), decimals)
                    .with_names(item.token.symbol, item.token.name);
                let (amount, cp) = direction(
                    owner,
                    item.from.as_ref().map(|a| a.hash.as_str()),
                    item.to.as_ref().map(|a| a.hash.as_str()),
                    amount,
                );
                let mut tx = RawTx::new(&item.transaction_hash, iso_ms(item.timestamp.as_deref()));
                tx.add(asset, amount, cp);
                txs.push(tx);
            }
        }

        if let Some(params) = &state.it {
            // Busy addresses sometimes time out here; internal value is a nice
            // to have, so a failure only ends this stream.
            if let Ok(page) = self
                .bs_get::<Listing<BsInternal>>(
                    &format!("{base}/api/v2/addresses/{owner}/internal-transactions"),
                    params,
                )
                .await
            {
                next.it = page.next_page_params;
                for item in page.items.into_iter().filter(|i| i.success != Some(false)) {
                    let value = item
                        .value
                        .as_deref()
                        .and_then(|v| from_base_units(v, 18))
                        .unwrap_or_default();
                    let (amount, cp) = direction(
                        owner,
                        item.from.as_ref().map(|a| a.hash.as_str()),
                        item.to.as_ref().map(|a| a.hash.as_str()),
                        value,
                    );
                    let mut tx =
                        RawTx::new(&item.transaction_hash, iso_ms(item.timestamp.as_deref()));
                    tx.add(native.clone(), amount, cp);
                    txs.push(tx);
                }
            }
        }

        let more = next.tx.is_some() || next.tt.is_some() || next.it.is_some();
        Ok(Page {
            txs: merge_by_hash(txs),
            next: more
                .then(|| serde_json::to_string(&next))
                .transpose()
                .map_err(|e| Error::provider(e.to_string()))?,
        })
    }

    async fn bs_get<T: for<'de> Deserialize<'de>>(&self, url: &str, params: &Value) -> Result<T> {
        let mut url = reqwest::Url::parse(url).map_err(|e| Error::provider(e.to_string()))?;
        if let Value::Object(map) = params {
            let mut pairs = url.query_pairs_mut();
            for (k, v) in map {
                let v = match v {
                    Value::String(s) => s.clone(),
                    other => other.to_string(),
                };
                pairs.append_pair(k, &v);
            }
        }
        let request = self.http.get(url);
        let response = request.send().await?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            // Addresses Blockscout has never seen.
            return serde_json::from_value(
                serde_json::json!({ "items": [], "next_page_params": null }),
            )
            .map_err(|e| Error::provider(e.to_string()));
        }
        Ok(response.error_for_status()?.json().await?)
    }
}

// ----------------------------------------------------------------- Etherscan

const ETHERSCAN_PAGE_SIZE: u32 = 100;

#[derive(Debug, Deserialize)]
struct EsResponse {
    status: String,
    message: String,
    result: Value,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EsTx {
    hash: String,
    time_stamp: String,
    from: String,
    to: String,
    value: String,
    gas_used: Option<String>,
    gas_price: Option<String>,
    is_error: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EsTokenTx {
    hash: String,
    time_stamp: String,
    from: String,
    to: String,
    value: String,
    contract_address: String,
    token_name: Option<String>,
    token_symbol: Option<String>,
    token_decimal: Option<String>,
}

impl EvmHistory {
    async fn etherscan(
        &self,
        base: &str,
        key: Option<&str>,
        owner: &str,
        cursor: Option<&str>,
    ) -> Result<Page> {
        let page: u32 = cursor.and_then(|c| c.parse().ok()).unwrap_or(1);
        let native = native_asset(self.chain);
        let mut txs = Vec::new();
        let mut full = false;

        for action in ["txlist", "tokentx", "txlistinternal"] {
            let mut url = format!(
                "{base}&module=account&action={action}&address={owner}&page={page}&offset={ETHERSCAN_PAGE_SIZE}&sort=desc"
            );
            if let Some(key) = key {
                url.push_str("&apikey=");
                url.push_str(key);
            }
            let response: EsResponse = self
                .http
                .get(&url)
                .send()
                .await?
                .error_for_status()?
                .json()
                .await?;
            let Value::Array(items) = response.result else {
                if response.status == "0" && response.message.starts_with("No transactions") {
                    continue;
                }
                return Err(Error::provider(format!(
                    "{}: {}",
                    response.message,
                    response.result.as_str().unwrap_or_default()
                )));
            };
            full |= items.len() as u32 >= ETHERSCAN_PAGE_SIZE;
            for item in items {
                match action {
                    "tokentx" => {
                        let Ok(t) = serde_json::from_value::<EsTokenTx>(item) else {
                            continue;
                        };
                        let decimals = t
                            .token_decimal
                            .as_deref()
                            .and_then(|d| d.parse().ok())
                            .unwrap_or(18);
                        let Some(amount) = from_base_units(&t.value, decimals) else {
                            continue;
                        };
                        let asset = AssetMeta::token(
                            self.chain,
                            t.contract_address.to_lowercase(),
                            decimals,
                        )
                        .with_names(t.token_symbol, t.token_name);
                        let (amount, cp) = direction(owner, Some(&t.from), Some(&t.to), amount);
                        let mut tx = RawTx::new(&t.hash, secs_ms(&t.time_stamp));
                        tx.add(asset, amount, cp);
                        txs.push(tx);
                    }
                    _ => {
                        let Ok(t) = serde_json::from_value::<EsTx>(item) else {
                            continue;
                        };
                        let mut tx = RawTx::new(&t.hash, secs_ms(&t.time_stamp));
                        tx.failed = t.is_error.as_deref() == Some("1");
                        if !tx.failed {
                            let value = from_base_units(&t.value, 18).unwrap_or_default();
                            let (amount, cp) = direction(owner, Some(&t.from), Some(&t.to), value);
                            tx.add(native.clone(), amount, cp);
                        }
                        if action == "txlist" && same(&t.from, owner) {
                            let fee = t
                                .gas_used
                                .as_deref()
                                .zip(t.gas_price.as_deref())
                                .and_then(|(used, price)| {
                                    let wei = used
                                        .parse::<u128>()
                                        .ok()?
                                        .checked_mul(price.parse::<u128>().ok()?)?;
                                    from_base_units(&wei.to_string(), 18)
                                })
                                .unwrap_or_default();
                            if !fee.is_zero() {
                                tx.fee = Some((native.clone(), fee));
                            }
                        }
                        txs.push(tx);
                    }
                }
            }
        }

        Ok(Page {
            txs: merge_by_hash(txs),
            next: full.then(|| (page + 1).to_string()),
        })
    }
}

fn secs_ms(seconds: &str) -> i64 {
    seconds.parse::<i64>().unwrap_or(0) * 1000
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_blockscout_timestamps() {
        assert_eq!(chrono_like_parse("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(
            chrono_like_parse("2024-02-29T12:34:56.789000Z"),
            Some(1_709_210_096_000)
        );
    }

    #[test]
    fn signs_amounts_by_direction() {
        let one = Decimal::ONE;
        assert_eq!(direction("0xA", Some("0xa"), Some("0xb"), one).0, -one);
        assert_eq!(direction("0xa", Some("0xb"), Some("0xA"), one).0, one);
        assert_eq!(
            direction("0xa", Some("0xa"), Some("0xa"), one).0,
            Decimal::ZERO
        );
        assert_eq!(
            direction("0xa", Some("0xb"), Some("0xc"), one).0,
            Decimal::ZERO
        );
    }

    #[test]
    fn picks_sources_by_chain_and_key() {
        let none = ApiKeys::default();
        assert!(matches!(
            source(Chain::Ethereum, &none),
            Some(Source::Blockscout(_))
        ));
        assert!(matches!(
            source(Chain::Avalanche, &none),
            Some(Source::Etherscan { key: None, .. })
        ));
        assert_eq!(source(Chain::Base, &none), None);
        let with_key = ApiKeys {
            etherscan: Some("k".into()),
        };
        assert!(matches!(
            source(Chain::Base, &with_key),
            Some(Source::Etherscan { key: Some(_), .. })
        ));
    }
}
