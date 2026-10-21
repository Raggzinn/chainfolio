//! Pure risk math: every factor is a 0–100 score where higher is riskier.

use rust_decimal::{Decimal, prelude::ToPrimitive};

/// Pegs further than this from $1 count as depegged.
pub const DEPEG_THRESHOLD: Decimal = Decimal::from_parts(1, 0, 0, false, 2);
/// A depeg this large (5%) counts at full severity.
const FULL_SEVERITY: Decimal = Decimal::from_parts(5, 0, 0, false, 2);

/// Herfindahl–Hirschman index of a set of values: the sum of squared shares,
/// from 1/n (evenly spread) to 1 (everything in one place).
pub fn hhi(values: &[Decimal]) -> Decimal {
    let total: Decimal = values.iter().filter(|v| v.is_sign_positive()).sum();
    if total.is_zero() {
        return Decimal::ZERO;
    }
    values
        .iter()
        .filter(|v| v.is_sign_positive())
        .map(|v| {
            let share = v / total;
            share * share
        })
        .sum::<Decimal>()
        .round_dp(6)
}

fn clamp_score(value: Decimal) -> u8 {
    value
        .round()
        .clamp(Decimal::ZERO, Decimal::ONE_HUNDRED)
        .to_u8()
        .unwrap_or(100)
}

fn ratio(numerator: Decimal, total: Decimal) -> Decimal {
    if total.is_zero() {
        Decimal::ZERO
    } else {
        numerator / total
    }
}

/// Blends the HHI with the largest single share, so one dominant asset scores
/// high even next to many small ones.
pub fn concentration(hhi: Decimal, top_share: Decimal) -> u8 {
    clamp_score(Decimal::ONE_HUNDRED * (Decimal::new(6, 1) * hhi + Decimal::new(4, 1) * top_share))
}

/// Distance of a stablecoin price from $1.
pub fn deviation(price: Decimal) -> Decimal {
    (price - Decimal::ONE).abs()
}

pub fn is_depegged(price: Decimal) -> bool {
    deviation(price) > DEPEG_THRESHOLD
}

/// Share of the portfolio in depegged stablecoins, weighted by how far off
/// the peg each one is. Half the portfolio 5% off peg scores 100.
pub fn depeg(stables: &[(Decimal, Decimal)], total: Decimal) -> u8 {
    let exposure: Decimal = stables
        .iter()
        .filter(|(price, _)| is_depegged(*price))
        .map(|(price, value)| value * (deviation(*price) / FULL_SEVERITY).min(Decimal::ONE))
        .sum();
    clamp_score(Decimal::from(200) * ratio(exposure, total))
}

/// Hidden spam adds a little; value held in risky tokens adds a lot.
pub fn suspicious(spam_count: usize, risky_value: Decimal, total: Decimal) -> u8 {
    clamp_score(Decimal::from(10 * spam_count) + Decimal::from(200) * ratio(risky_value, total))
}

pub fn approvals(unlimited: usize, limited: usize) -> u8 {
    clamp_score(Decimal::from(15 * unlimited + 5 * limited))
}

/// Liquidation risk from the lowest lending health factor.
pub fn liquidation(min_health_factor: Option<Decimal>) -> u8 {
    let Some(hf) = min_health_factor else {
        return 0;
    };
    match hf {
        hf if hf < Decimal::new(11, 1) => 100,
        hf if hf < Decimal::new(125, 2) => 75,
        hf if hf < Decimal::new(15, 1) => 50,
        hf if hf < Decimal::TWO => 25,
        _ => 0,
    }
}

/// Shares of the portfolio exposed to smart contracts and to single points
/// of failure, as fractions of the total.
pub struct ExposureShares {
    pub defi: Decimal,
    pub top_protocol: Decimal,
    pub top_chain: Decimal,
}

/// Smart-contract and chain exposure, or liquidation risk if that is worse.
pub fn exposure(shares: &ExposureShares, min_health_factor: Option<Decimal>) -> u8 {
    let blended = Decimal::new(35, 2) * shares.defi
        + Decimal::new(35, 2) * shares.top_protocol
        + Decimal::new(3, 1) * shares.top_chain;
    clamp_score(Decimal::ONE_HUNDRED * blended).max(liquidation(min_health_factor))
}

/// Weighted average of factor scores; weights need not sum to one.
pub fn overall(factors: &[(u8, Decimal)]) -> u8 {
    let weights: Decimal = factors.iter().map(|(_, w)| w).sum();
    let weighted: Decimal = factors.iter().map(|(s, w)| Decimal::from(*s) * w).sum();
    clamp_score(ratio(weighted, weights))
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    fn d(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    #[test]
    fn computes_hhi() {
        assert_eq!(hhi(&[d("100")]), Decimal::ONE);
        assert_eq!(hhi(&[d("50"), d("50")]), d("0.5"));
        assert_eq!(hhi(&[d("1"); 4]), d("0.25"));
        // 70/20/10 → 0.49 + 0.04 + 0.01
        assert_eq!(hhi(&[d("700"), d("200"), d("100")]), d("0.54"));
        assert_eq!(hhi(&[]), Decimal::ZERO);
        assert_eq!(hhi(&[d("-5"), d("10")]), Decimal::ONE);
    }

    #[test]
    fn scores_concentration() {
        assert_eq!(concentration(Decimal::ONE, Decimal::ONE), 100);
        assert_eq!(concentration(d("0.25"), d("0.25")), 25);
        assert_eq!(concentration(d("0.54"), d("0.7")), 60);
    }

    #[test]
    fn detects_depegs() {
        assert!(!is_depegged(d("0.9995")));
        assert!(!is_depegged(d("1.01")));
        assert!(is_depegged(d("0.985")));
        assert!(is_depegged(d("1.02")));
        assert_eq!(deviation(d("0.97")), d("0.03"));
        // Pegged stables don't count, however large.
        assert_eq!(depeg(&[(d("1.0002"), d("1000"))], d("1000")), 0);
        // 10% of the portfolio 3% off peg: 200 * 0.1 * 0.6
        assert_eq!(depeg(&[(d("0.97"), d("100"))], d("1000")), 12);
        // Severity caps at 5% off peg.
        assert_eq!(depeg(&[(d("0.5"), d("600"))], d("1000")), 100);
        assert_eq!(depeg(&[], Decimal::ZERO), 0);
    }

    #[test]
    fn scores_the_rest() {
        assert_eq!(suspicious(0, Decimal::ZERO, d("1000")), 0);
        assert_eq!(suspicious(3, d("50"), d("1000")), 40);
        assert_eq!(approvals(2, 1), 35);
        assert_eq!(approvals(10, 0), 100);
        assert_eq!(liquidation(None), 0);
        assert_eq!(liquidation(Some(d("1.05"))), 100);
        assert_eq!(liquidation(Some(d("1.8"))), 25);
        let shares = |defi, protocol, chain| ExposureShares {
            defi: d(defi),
            top_protocol: d(protocol),
            top_chain: d(chain),
        };
        // 35 * 0.4 + 35 * 0.2 + 30 * 0.5
        assert_eq!(exposure(&shares("0.4", "0.2", "0.5"), Some(d("3"))), 36);
        assert_eq!(exposure(&shares("0.1", "0.1", "0.2"), Some(d("1.2"))), 75);
        assert_eq!(exposure(&shares("0", "0", "1"), None), 30);
        assert_eq!(overall(&[(100, d("0.5")), (0, d("0.5"))]), 50);
        assert_eq!(overall(&[(40, d("3")), (80, d("1"))]), 50);
    }
}
