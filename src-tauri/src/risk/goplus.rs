//! GoPlus token security API (keyless): honeypots, transfer taxes, blacklist
//! functions and airdrop scams for EVM tokens. Without a key it answers one
//! contract per request and throttles bursts after ~8 calls (code 4029).
//! https://docs.gopluslabs.io/reference/api-overview

use std::collections::HashMap;

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::{chains::Chain, error::Result};

const API: &str = "https://api.gopluslabs.io/api/v1/token_security";
/// GoPlus' "too many requests" code.
const RATE_LIMITED: i64 = 4029;
/// Taxes above this (10%) are flagged; above 50% the token is treated as spam.
const HIGH_TAX: Decimal = Decimal::from_parts(1, 0, 0, false, 1);
const SPAM_TAX: Decimal = Decimal::from_parts(5, 0, 0, false, 1);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Reason {
    Honeypot,
    CannotSellAll,
    AirdropScam,
    HighTax,
    Blacklist,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Verdict {
    pub reasons: Vec<Reason>,
    /// Hidden from the portfolio: the token is a scam, not merely risky.
    pub spam: bool,
}

#[derive(Debug, Deserialize)]
struct Response {
    code: i64,
    message: Option<String>,
    result: Option<HashMap<String, TokenSecurity>>,
}

#[derive(Debug, Default, Deserialize)]
struct TokenSecurity {
    is_honeypot: Option<String>,
    cannot_sell_all: Option<String>,
    is_airdrop_scam: Option<String>,
    is_blacklisted: Option<String>,
    buy_tax: Option<String>,
    sell_tax: Option<String>,
    trust_list: Option<String>,
}

fn flag(value: &Option<String>) -> bool {
    value.as_deref() == Some("1")
}

fn tax(value: &Option<String>) -> Decimal {
    value
        .as_deref()
        .and_then(|v| v.parse::<Decimal>().ok())
        .unwrap_or_default()
}

fn verdict(security: &TokenSecurity) -> Verdict {
    // Tokens on GoPlus' trust list (USDT, USDC…) keep admin features such as
    // blacklisting by design; they are not what this scan looks for.
    if flag(&security.trust_list) {
        return Verdict::default();
    }
    let max_tax = tax(&security.buy_tax).max(tax(&security.sell_tax));
    let mut reasons = Vec::new();
    for (hit, reason) in [
        (flag(&security.is_honeypot), Reason::Honeypot),
        (flag(&security.cannot_sell_all), Reason::CannotSellAll),
        (flag(&security.is_airdrop_scam), Reason::AirdropScam),
        (max_tax > HIGH_TAX, Reason::HighTax),
        (flag(&security.is_blacklisted), Reason::Blacklist),
    ] {
        if hit {
            reasons.push(reason);
        }
    }
    let spam = reasons.iter().any(|r| {
        matches!(
            r,
            Reason::Honeypot | Reason::CannotSellAll | Reason::AirdropScam
        )
    }) || max_tax >= SPAM_TAX;
    Verdict { reasons, spam }
}

/// GoPlus chain id; only EVM chains are covered.
pub fn chain_id(chain: Chain) -> Option<u64> {
    Some(match chain {
        Chain::Ethereum => 1,
        Chain::Optimism => 10,
        Chain::Bsc => 56,
        Chain::Polygon => 137,
        Chain::Base => 8453,
        Chain::Arbitrum => 42161,
        Chain::Avalanche => 43114,
        _ => return None,
    })
}

pub enum Outcome {
    Checked(Option<Verdict>),
    /// Throttled; try the remaining tokens on a later scan.
    RateLimited,
}

/// Verdict for one contract; `None` when GoPlus doesn't know the token.
pub async fn check(http: &reqwest::Client, chain: Chain, contract: &str) -> Result<Outcome> {
    let Some(id) = chain_id(chain) else {
        return Ok(Outcome::Checked(None));
    };
    let response: Response = http
        .get(format!("{API}/{id}?contract_addresses={contract}"))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    if response.code == RATE_LIMITED {
        return Ok(Outcome::RateLimited);
    }
    let mut verdicts = parse(response)?;
    Ok(Outcome::Checked(verdicts.remove(&contract.to_lowercase())))
}

fn parse(response: Response) -> Result<HashMap<String, Verdict>> {
    if response.code != 1 {
        return Err(crate::error::Error::provider(format!(
            "GoPlus: {}",
            response
                .message
                .unwrap_or_else(|| response.code.to_string())
        )));
    }
    Ok(response
        .result
        .unwrap_or_default()
        .into_iter()
        .map(|(address, security)| (address.to_lowercase(), verdict(&security)))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_risky_tokens() {
        let body = r#"{"code": 1, "message": "OK", "result": {
          "0xdac17f958d2ee523a2206206994597c13d831ec7": {"is_blacklisted": "1", "is_honeypot": "0",
            "buy_tax": "0", "sell_tax": "0", "trust_list": "1", "token_symbol": "USDT"},
          "0xHONEY": {"is_honeypot": "1", "buy_tax": "0", "sell_tax": "1"},
          "0xtax": {"is_honeypot": "0", "buy_tax": "0.12", "sell_tax": "0.15", "is_blacklisted": "1"},
          "0xclean": {"is_honeypot": "0", "buy_tax": "", "sell_tax": "", "is_airdrop_scam": "0"},
          "0xairdrop": {"is_airdrop_scam": "1"}
        }}"#;
        let verdicts = parse(serde_json::from_str(body).unwrap()).unwrap();
        assert_eq!(
            verdicts["0xdac17f958d2ee523a2206206994597c13d831ec7"],
            Verdict::default()
        );
        let honey = &verdicts["0xhoney"];
        assert!(honey.spam);
        assert_eq!(honey.reasons, vec![Reason::Honeypot, Reason::HighTax]);
        let taxed = &verdicts["0xtax"];
        assert!(!taxed.spam);
        assert_eq!(taxed.reasons, vec![Reason::HighTax, Reason::Blacklist]);
        assert_eq!(verdicts["0xclean"], Verdict::default());
        assert!(verdicts["0xairdrop"].spam);
    }

    #[test]
    fn reports_api_errors() {
        let body = r#"{"code": 4029, "message": "too many requests", "result": {}}"#;
        assert!(parse(serde_json::from_str(body).unwrap()).is_err());
    }
}
