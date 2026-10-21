//! Native Solana staking: stake accounts whose withdraw authority is the
//! wallet, found with `getProgramAccounts` on the stake program.

use async_trait::async_trait;
use rust_decimal::Decimal;
use serde::Deserialize;
use serde_json::json;

use super::{Position, PositionAdapter, PositionKind, PositionToken, TokenRole};
use crate::{
    chains::{Chain, rpc, solana::RPCS},
    error::Result,
};

const STAKE_PROGRAM: &str = "Stake11111111111111111111111111111111111111";
/// Byte offset of `Meta.authorized.withdrawer` in a stake account
/// (4-byte state tag, 8-byte rent reserve, 32-byte staker).
const WITHDRAWER_OFFSET: u64 = 44;

pub struct SolanaStake {
    http: reqwest::Client,
}

impl SolanaStake {
    pub fn new(http: reqwest::Client) -> Self {
        Self { http }
    }
}

#[derive(Deserialize)]
struct StakeAccount {
    account: AccountInfo,
}

#[derive(Deserialize)]
struct AccountInfo {
    lamports: u64,
    data: Option<serde_json::Value>,
}

impl StakeAccount {
    /// Delegated (or delegating) accounts as opposed to merely initialized.
    fn is_delegated(&self) -> bool {
        self.account
            .data
            .as_ref()
            .and_then(|d| d.pointer("/parsed/type"))
            .and_then(|t| t.as_str())
            == Some("delegated")
    }
}

/// One position summing every stake account of the wallet.
fn stake_position(accounts: &[StakeAccount]) -> Option<Position> {
    let lamports: u64 = accounts.iter().map(|a| a.account.lamports).sum();
    if lamports == 0 {
        return None;
    }
    let delegated = accounts.iter().filter(|a| a.is_delegated()).count();
    let mut position = Position::new(
        Chain::Solana,
        "solana-staking",
        PositionKind::Staking,
        format!("{delegated}/{}", accounts.len()),
    );
    position.tokens.push(PositionToken {
        symbol: "SOL".into(),
        contract: None,
        price_key: "coingecko:solana".into(),
        role: TokenRole::Staked,
        amount: Decimal::from_i128_with_scale(i128::from(lamports), 9).normalize(),
        value_usd: None,
        apy: None,
        liquidation_threshold: None,
    });
    Some(position)
}

#[async_trait]
impl PositionAdapter for SolanaStake {
    fn source(&self) -> String {
        "solana-staking:solana".into()
    }

    fn chain(&self) -> Chain {
        Chain::Solana
    }

    async fn positions(&self, owner: &str) -> Result<Vec<Position>> {
        let accounts: Vec<StakeAccount> = rpc::call(
            &self.http,
            RPCS,
            "getProgramAccounts",
            json!([STAKE_PROGRAM, {
                "encoding": "jsonParsed",
                "filters": [{ "memcmp": { "offset": WITHDRAWER_OFFSET, "bytes": owner } }],
            }]),
        )
        .await?;
        Ok(stake_position(&accounts).into_iter().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sums_stake_accounts() {
        let body = r#"[
          {"pubkey": "a", "account": {"lamports": 2500000000, "owner": "Stake11111111111111111111111111111111111111",
            "data": {"program": "stake", "parsed": {"type": "delegated", "info": {}}}}},
          {"pubkey": "b", "account": {"lamports": 1002282880,
            "data": {"program": "stake", "parsed": {"type": "initialized", "info": {}}}}}
        ]"#;
        let accounts: Vec<StakeAccount> = serde_json::from_str(body).unwrap();
        let position = stake_position(&accounts).unwrap();
        assert_eq!(position.tokens[0].amount, Decimal::new(350_228_288, 8));
        assert_eq!(position.name, "1/2");
        assert!(stake_position(&[]).is_none());
    }
}
