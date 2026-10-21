//! Liquid staking tokens. They are ordinary balances, so their positions are
//! derived from stored holdings and flagged as already counted.

use rust_decimal::Decimal;

use super::{Position, PositionKind, PositionToken, TokenRole, yields::Yields};
use crate::chains::Chain;

pub struct Lst {
    pub chain: Chain,
    /// Contract (lowercase) or mint.
    pub contract: &'static str,
    pub protocol: &'static str,
    /// DefiLlama yields project and the chain its APY is reported on.
    pub yield_project: &'static str,
    pub yield_chain: &'static str,
}

const fn lst(
    chain: Chain,
    contract: &'static str,
    protocol: &'static str,
    yield_project: &'static str,
    yield_chain: &'static str,
) -> Lst {
    Lst {
        chain,
        contract,
        protocol,
        yield_project,
        yield_chain,
    }
}

const LIDO: &str = "lido";
const ETH: &str = "Ethereum";

pub const LSTS: &[Lst] = &[
    lst(
        Chain::Ethereum,
        "0xae7ab96520de3a18e5e111b5eaab095312d7fe84",
        LIDO,
        LIDO,
        ETH,
    ),
    lst(
        Chain::Ethereum,
        "0x7f39c581f595b53c5cb19bd0b3f8da6c935e2ca0",
        LIDO,
        LIDO,
        ETH,
    ),
    lst(
        Chain::Arbitrum,
        "0x5979d7b546e38e414f7e9822514be443a4800529",
        LIDO,
        LIDO,
        ETH,
    ),
    lst(
        Chain::Base,
        "0xc1cba3fcea344f92d9239c08c0568f6f2f0ee452",
        LIDO,
        LIDO,
        ETH,
    ),
    lst(
        Chain::Optimism,
        "0x1f32b1c2345538c0c6f582fcb022739c4a194ebb",
        LIDO,
        LIDO,
        ETH,
    ),
    lst(
        Chain::Ethereum,
        "0xae78736cd615f374d3085123a210448e74fc6393",
        "rocket-pool",
        "rocket-pool",
        ETH,
    ),
    lst(
        Chain::Ethereum,
        "0xbe9895146f7af43049ca1c1ae358b0541ea49704",
        "coinbase",
        "coinbase-wrapped-staked-eth",
        ETH,
    ),
    lst(
        Chain::Base,
        "0x2ae3f1ec7f1f5012cfeab0185bfc7aa3cf0dec22",
        "coinbase",
        "coinbase-wrapped-staked-eth",
        ETH,
    ),
    lst(
        Chain::Solana,
        "mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So",
        "marinade",
        "marinade-liquid-staking",
        "Solana",
    ),
    lst(
        Chain::Solana,
        "J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn",
        "jito",
        "jito-liquid-staking",
        "Solana",
    ),
    lst(
        Chain::Avalanche,
        "0x2b2c81e08f1af8835a78bb2a90ae924ace0ea4be",
        "benqi",
        "benqi-staked-avax",
        "Avalanche",
    ),
];

/// The liquid staking token stored under `asset_id` (`<chain>:<contract>`).
pub fn find(asset_id: &str) -> Option<&'static Lst> {
    LSTS.iter()
        .find(|l| asset_id == format!("{}:{}", l.chain, l.contract))
}

/// A staking position mirroring a held liquid staking token.
pub fn position(
    lst: &Lst,
    symbol: &str,
    price_key: &str,
    amount: Decimal,
    yields: Option<&Yields>,
) -> Position {
    let mut position = Position::new(
        lst.chain,
        lst.protocol,
        PositionKind::Staking,
        symbol.to_string(),
    );
    position.in_balances = true;
    let mut token = PositionToken::new(
        lst.chain,
        lst.contract,
        symbol.to_string(),
        TokenRole::Staked,
        amount,
    );
    token.price_key = price_key.to_string();
    token.apy = yields.and_then(|y| y.staking(lst.yield_project, lst.yield_chain));
    position.tokens.push(token);
    position
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_lsts_by_asset_id() {
        let steth = find("ethereum:0xae7ab96520de3a18e5e111b5eaab095312d7fe84").unwrap();
        assert_eq!(steth.protocol, "lido");
        assert!(find("solana:J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn").is_some());
        // Mints are case-sensitive.
        assert!(find("solana:j1toso1uck3rlmjorhttrvwy9hj7x8v9yyac6y7kgcpn").is_none());
        assert!(find("ethereum:0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48").is_none());
    }

    #[test]
    fn mirrors_the_balance() {
        let lst = find("base:0x2ae3f1ec7f1f5012cfeab0185bfc7aa3cf0dec22").unwrap();
        let p = position(lst, "cbETH", "base:0x2ae3", Decimal::TWO, None);
        assert!(p.in_balances);
        assert_eq!(p.kind, PositionKind::Staking);
        assert_eq!(p.tokens[0].price_key, "base:0x2ae3");
        assert_eq!(p.tokens[0].amount, Decimal::TWO);
    }
}
