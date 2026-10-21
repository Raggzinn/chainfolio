//! Minimal unsigned 256-bit integer for ABI words: comparisons, shifts,
//! modular add/sub and the 128×128-bit multiply Uniswap fee math needs.

use std::{
    fmt,
    ops::{BitAnd, BitOr, Shl, Shr, Sub},
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct U256 {
    // Field order makes the derived ordering numeric.
    hi: u128,
    lo: u128,
}

impl U256 {
    pub const ZERO: Self = Self { hi: 0, lo: 0 };
    pub const MAX: Self = Self {
        hi: u128::MAX,
        lo: u128::MAX,
    };

    pub const fn from_parts(hi: u128, lo: u128) -> Self {
        Self { hi, lo }
    }

    pub fn from_be_bytes(bytes: [u8; 32]) -> Self {
        let (hi, lo) = bytes.split_at(16);
        Self {
            hi: u128::from_be_bytes(hi.try_into().expect("16 bytes")),
            lo: u128::from_be_bytes(lo.try_into().expect("16 bytes")),
        }
    }

    pub fn to_be_bytes(self) -> [u8; 32] {
        let mut out = [0u8; 32];
        out[..16].copy_from_slice(&self.hi.to_be_bytes());
        out[16..].copy_from_slice(&self.lo.to_be_bytes());
        out
    }

    pub fn is_zero(self) -> bool {
        self.hi == 0 && self.lo == 0
    }

    pub fn bit(self, i: usize) -> bool {
        match i {
            0..128 => self.lo >> i & 1 == 1,
            128..256 => self.hi >> (i - 128) & 1 == 1,
            _ => false,
        }
    }

    pub fn wrapping_add(self, rhs: Self) -> Self {
        let (lo, carry) = self.lo.overflowing_add(rhs.lo);
        Self {
            hi: self.hi.wrapping_add(rhs.hi).wrapping_add(u128::from(carry)),
            lo,
        }
    }

    pub fn wrapping_sub(self, rhs: Self) -> Self {
        let (lo, borrow) = self.lo.overflowing_sub(rhs.lo);
        Self {
            hi: self
                .hi
                .wrapping_sub(rhs.hi)
                .wrapping_sub(u128::from(borrow)),
            lo,
        }
    }

    /// Full 256-bit product of two 128-bit values.
    pub fn mul_wide(a: u128, b: u128) -> Self {
        const MASK: u128 = u64::MAX as u128;
        let (a1, a0) = (a >> 64, a & MASK);
        let (b1, b0) = (b >> 64, b & MASK);
        let p00 = a0 * b0;
        let p01 = a0 * b1;
        let p10 = a1 * b0;
        let p11 = a1 * b1;
        let middle = (p00 >> 64) + (p01 & MASK) + (p10 & MASK);
        Self {
            hi: p11 + (p01 >> 64) + (p10 >> 64) + (middle >> 64),
            lo: (middle << 64) | (p00 & MASK),
        }
    }

    /// `self * y / 2^128` rounded down, like Solidity's `FullMath.mulDiv(x,
    /// y, Q128)` for `y < 2^128`; wraps on overflow.
    pub fn mul_shr_128(self, y: u128) -> Self {
        Self::mul_wide(self.hi, y).wrapping_add(Self::from(Self::mul_wide(self.lo, y).hi))
    }

    /// Quotient and remainder of a division by a small divisor.
    fn div_rem_u64(self, d: u64) -> (Self, u64) {
        let d = u128::from(d);
        let limbs = [
            (self.hi >> 64) as u64,
            self.hi as u64,
            (self.lo >> 64) as u64,
            self.lo as u64,
        ];
        let mut q = [0u64; 4];
        let mut rem = 0u128;
        for (i, limb) in limbs.into_iter().enumerate() {
            let cur = (rem << 64) | u128::from(limb);
            q[i] = (cur / d) as u64;
            rem = cur % d;
        }
        let quotient = Self {
            hi: (u128::from(q[0]) << 64) | u128::from(q[1]),
            lo: (u128::from(q[2]) << 64) | u128::from(q[3]),
        };
        (quotient, rem as u64)
    }

    /// Lossy conversion for display math.
    pub fn to_f64(self) -> f64 {
        self.hi as f64 * 2f64.powi(128) + self.lo as f64
    }
}

impl From<u128> for U256 {
    fn from(lo: u128) -> Self {
        Self { hi: 0, lo }
    }
}

impl From<u64> for U256 {
    fn from(n: u64) -> Self {
        Self::from(u128::from(n))
    }
}

impl From<u32> for U256 {
    fn from(n: u32) -> Self {
        Self::from(u128::from(n))
    }
}

impl From<u8> for U256 {
    fn from(n: u8) -> Self {
        Self::from(u128::from(n))
    }
}

macro_rules! try_into_primitive {
    ($($t:ty),*) => {$(
        impl TryFrom<U256> for $t {
            type Error = std::num::TryFromIntError;

            fn try_from(n: U256) -> Result<Self, Self::Error> {
                // A non-zero high half can't fit; `u8::try_from(256u16)`
                // produces the matching error value.
                if n.hi != 0 {
                    return Err(u8::try_from(256u16).unwrap_err());
                }
                <$t>::try_from(n.lo)
            }
        }
    )*};
}

try_into_primitive!(u32, u64, usize);

impl TryFrom<U256> for u128 {
    type Error = std::num::TryFromIntError;

    fn try_from(n: U256) -> Result<Self, Self::Error> {
        if n.hi != 0 {
            return Err(u8::try_from(256u16).unwrap_err());
        }
        Ok(n.lo)
    }
}

impl Shl<usize> for U256 {
    type Output = Self;

    fn shl(self, n: usize) -> Self {
        match n {
            0 => self,
            1..128 => Self {
                hi: (self.hi << n) | (self.lo >> (128 - n)),
                lo: self.lo << n,
            },
            128..256 => Self {
                hi: self.lo << (n - 128),
                lo: 0,
            },
            _ => Self::ZERO,
        }
    }
}

impl Shr<usize> for U256 {
    type Output = Self;

    fn shr(self, n: usize) -> Self {
        match n {
            0 => self,
            1..128 => Self {
                hi: self.hi >> n,
                lo: (self.lo >> n) | (self.hi << (128 - n)),
            },
            128..256 => Self {
                hi: 0,
                lo: self.hi >> (n - 128),
            },
            _ => Self::ZERO,
        }
    }
}

impl BitAnd for U256 {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self {
        Self {
            hi: self.hi & rhs.hi,
            lo: self.lo & rhs.lo,
        }
    }
}

impl BitOr for U256 {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self {
        Self {
            hi: self.hi | rhs.hi,
            lo: self.lo | rhs.lo,
        }
    }
}

/// Modular subtraction, as in Solidity's `unchecked` blocks.
impl Sub for U256 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        self.wrapping_sub(rhs)
    }
}

impl fmt::Display for U256 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.hi == 0 {
            return write!(f, "{}", self.lo);
        }
        // Peel off 19 decimal digits at a time (10^19 fits in a u64).
        const CHUNK: u64 = 10_000_000_000_000_000_000;
        let mut chunks = Vec::new();
        let mut n = *self;
        while !n.is_zero() {
            let (q, r) = n.div_rem_u64(CHUNK);
            chunks.push(r);
            n = q;
        }
        let mut out = chunks.pop().unwrap_or(0).to_string();
        for chunk in chunks.into_iter().rev() {
            out.push_str(&format!("{chunk:019}"));
        }
        f.write_str(&out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_decimals() {
        assert_eq!(U256::ZERO.to_string(), "0");
        assert_eq!(U256::from(u128::MAX).to_string(), u128::MAX.to_string());
        assert_eq!(
            U256::MAX.to_string(),
            "115792089237316195423570985008687907853269984665640564039457584007913129639935"
        );
        assert_eq!(
            (U256::from(1u8) << 128).to_string(),
            "340282366920938463463374607431768211456"
        );
    }

    #[test]
    fn shifts_and_masks() {
        let one = U256::from(1u8);
        assert_eq!((one << 200) >> 200, one);
        assert_eq!((one << 96) - one, U256::from((1u128 << 96) - 1));
        assert!((one << 130).bit(130));
        assert!(!(one << 130).bit(2));
        assert_eq!(
            (U256::from(0xabcdu64) >> 8) & U256::from(0xffu8),
            U256::from(0xabu8)
        );
        assert_eq!(one << 256, U256::ZERO);
    }

    #[test]
    fn wraps_around() {
        let one = U256::from(1u8);
        assert_eq!(U256::ZERO.wrapping_sub(one), U256::MAX);
        assert_eq!(U256::MAX.wrapping_add(one), U256::ZERO);
        assert_eq!(U256::from(u128::MAX).wrapping_add(one), one << 128);
        assert_eq!((one << 128).wrapping_sub(one), U256::from(u128::MAX));
    }

    #[test]
    fn multiplies_wide() {
        let p = U256::mul_wide(u128::MAX, u128::MAX);
        // (2^128 - 1)^2 = 2^256 - 2^129 + 1
        assert_eq!(
            p,
            U256::ZERO
                .wrapping_sub(U256::from(1u8) << 129)
                .wrapping_add(U256::from(1u8))
        );
        assert_eq!(U256::mul_wide(3, 5), U256::from(15u8));
        // (7 * 2^128 + 2^127) * 4 / 2^128 = 30
        let x = U256::from_parts(7, 1u128 << 127);
        assert_eq!(x.mul_shr_128(4), U256::from(30u8));
    }

    #[test]
    fn round_trips_bytes_and_converts() {
        let n = U256::from_parts(42, 7);
        assert_eq!(U256::from_be_bytes(n.to_be_bytes()), n);
        assert!(u64::try_from(n).is_err());
        assert_eq!(u32::try_from(U256::from(9u8)).unwrap(), 9);
        assert!(U256::from(2u8) < U256::from_parts(1, 0));
        assert_eq!(U256::from_parts(1, 0).to_f64(), 2f64.powi(128));
    }
}
