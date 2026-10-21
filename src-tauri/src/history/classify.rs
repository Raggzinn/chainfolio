//! Turns the balance changes of a transaction into a human category, per
//! wallet. Everything here is pure so it can be tested in isolation.

use std::collections::{HashMap, HashSet};

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Transaction categories. The auto classifier only produces the first six;
/// the rest are labels a user can pick to refine one (they matter for PnL and
/// taxes: income is acquired at market value, a sale realizes a gain…).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Receive,
    Send,
    Swap,
    Internal,
    Contract,
    Failed,
    Income,
    Reward,
    Airdrop,
    Gift,
    Sale,
    Spam,
}

impl Kind {
    pub const ALL: [Kind; 12] = [
        Kind::Receive,
        Kind::Send,
        Kind::Swap,
        Kind::Internal,
        Kind::Contract,
        Kind::Failed,
        Kind::Income,
        Kind::Reward,
        Kind::Airdrop,
        Kind::Gift,
        Kind::Sale,
        Kind::Spam,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Receive => "receive",
            Kind::Send => "send",
            Kind::Swap => "swap",
            Kind::Internal => "internal",
            Kind::Contract => "contract",
            Kind::Failed => "failed",
            Kind::Income => "income",
            Kind::Reward => "reward",
            Kind::Airdrop => "airdrop",
            Kind::Gift => "gift",
            Kind::Sale => "sale",
            Kind::Spam => "spam",
        }
    }

    pub fn parse(s: &str) -> Option<Kind> {
        Kind::ALL.into_iter().find(|k| k.as_str() == s)
    }
}

/// What one transaction did to one wallet.
#[derive(Clone, Debug, Default)]
pub struct EntryFacts {
    /// Net change per asset id, fees excluded.
    pub deltas: Vec<(String, Decimal)>,
    /// Counterparty addresses of the transfers.
    pub counterparties: Vec<String>,
    pub failed: bool,
}

/// Classifies every wallet's view of one transaction. `own_addresses` are all
/// tracked addresses (any case); a transfer to or from one of them is internal.
pub fn classify(
    entries: &HashMap<i64, EntryFacts>,
    own_addresses: &HashSet<String>,
) -> HashMap<i64, Kind> {
    entries
        .iter()
        .map(|(&wallet, facts)| (wallet, classify_one(wallet, facts, entries, own_addresses)))
        .collect()
}

fn classify_one(
    wallet: i64,
    facts: &EntryFacts,
    all: &HashMap<i64, EntryFacts>,
    own: &HashSet<String>,
) -> Kind {
    if facts.failed {
        return Kind::Failed;
    }
    let ins: Vec<&(String, Decimal)> = facts
        .deltas
        .iter()
        .filter(|(_, a)| a.is_sign_positive() && !a.is_zero())
        .collect();
    let outs: Vec<&(String, Decimal)> = facts
        .deltas
        .iter()
        .filter(|(_, a)| a.is_sign_negative())
        .collect();

    if ins.is_empty() && outs.is_empty() {
        return Kind::Contract;
    }

    // Moved between two tracked wallets: the other side holds the mirror image.
    let mirrored = |(asset, amount): &(String, Decimal)| {
        all.iter().any(|(&other, f)| {
            other != wallet
                && f.deltas
                    .iter()
                    .any(|(a, b)| a == asset && b.is_sign_positive() != amount.is_sign_positive())
        })
    };
    let to_own = facts
        .counterparties
        .iter()
        .any(|c| own.contains(&c.to_lowercase()));
    let single_direction = ins.is_empty() || outs.is_empty();
    if single_direction && (to_own || facts.deltas.iter().all(mirrored)) {
        return Kind::Internal;
    }

    let in_assets: HashSet<&str> = ins.iter().map(|(a, _)| a.as_str()).collect();
    let out_assets: HashSet<&str> = outs.iter().map(|(a, _)| a.as_str()).collect();
    match (ins.is_empty(), outs.is_empty()) {
        (false, false) if in_assets != out_assets => Kind::Swap,
        (false, false) => {
            // Same asset both ways (e.g. a refund): label by the net result.
            let net: Decimal = facts.deltas.iter().map(|(_, a)| *a).sum();
            if net.is_sign_negative() {
                Kind::Send
            } else {
                Kind::Receive
            }
        }
        (false, true) => Kind::Receive,
        (true, false) => Kind::Send,
        (true, true) => Kind::Contract,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(asset: &str, amount: i64) -> (String, Decimal) {
        (asset.to_string(), Decimal::from(amount))
    }

    fn facts(deltas: Vec<(String, Decimal)>) -> EntryFacts {
        EntryFacts {
            deltas,
            ..EntryFacts::default()
        }
    }

    fn one(f: EntryFacts) -> Kind {
        let entries = HashMap::from([(1, f)]);
        classify(&entries, &HashSet::new())[&1]
    }

    #[test]
    fn basic_directions() {
        assert_eq!(one(facts(vec![d("eth", 1)])), Kind::Receive);
        assert_eq!(one(facts(vec![d("eth", -1)])), Kind::Send);
        assert_eq!(one(facts(vec![d("eth", -1), d("usdc", 3000)])), Kind::Swap);
        assert_eq!(one(facts(vec![])), Kind::Contract);
    }

    #[test]
    fn failed_wins() {
        let mut f = facts(vec![d("eth", -1)]);
        f.failed = true;
        assert_eq!(one(f), Kind::Failed);
    }

    #[test]
    fn transfer_between_tracked_wallets_is_internal() {
        let entries = HashMap::from([
            (1, facts(vec![d("eth", -2)])),
            (2, facts(vec![d("eth", 2)])),
        ]);
        let kinds = classify(&entries, &HashSet::new());
        assert_eq!(kinds[&1], Kind::Internal);
        assert_eq!(kinds[&2], Kind::Internal);
    }

    #[test]
    fn counterparty_that_is_tracked_marks_internal() {
        let mut f = facts(vec![d("btc", -1)]);
        f.counterparties = vec!["0xABC".into()];
        let own = HashSet::from(["0xabc".to_string()]);
        let kinds = classify(&HashMap::from([(1, f)]), &own);
        assert_eq!(kinds[&1], Kind::Internal);
    }

    #[test]
    fn kinds_round_trip() {
        for kind in Kind::ALL {
            assert_eq!(Kind::parse(kind.as_str()), Some(kind));
        }
    }
}
