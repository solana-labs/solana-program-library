//! Math for preserving precision

#![allow(clippy::assign_op_pattern)]
#![allow(clippy::ptr_offset_with_cast)]
#![allow(clippy::reversed_empty_ranges)]
#![allow(clippy::manual_range_contains)]

use crate::math::common::*;
use std::fmt;
use uint::construct_uint;

// U128 with 128 bits consisting of 2 x 64-bit words
construct_uint! {
    pub struct U128(2);
}

/// Small decimal value precise to 18 digits
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd, Eq, Ord)]
pub struct Rate(pub U128);

impl Rate {
    /// One
    pub fn one() -> Self {
        Self(Self::wad())
    }

    /// Zero
    pub fn zero() -> Self {
        Self(U128::from(0))
    }

    // TODO: use const slices when fixed
    fn wad() -> U128 {
        U128::from(WAD)
    }

    // TODO: use const slices when fixed
    fn half_wad() -> U128 {
        U128::from(HALF_WAD)
    }

    /// Create scaled decimal from percent value
    pub fn from_percent(percent: u8) -> Self {
        Self(U128::from(percent as u64 * PERCENT_SCALER))
    }

    /// Create scaled decimal from value and scale
    pub fn new(val: u64, scale: usize) -> Self {
        assert!(scale <= SCALE);
        Self(Self::wad() / U128::exp10(scale) * U128::from(val))
    }

    /// Return raw scaled value
    pub fn to_scaled_val(&self) -> u128 {
        self.0.as_u128()
    }

    /// Create decimal from scaled value
    pub fn from_scaled_val(scaled_val: u128) -> Self {
        Self(U128::from(scaled_val))
    }

    /// Round scaled decimal to u64
    pub fn round_u64(&self) -> u64 {
        ((Self::half_wad() + self.0) / Self::wad()).as_u64()
    }

    /// Calculates base^exp
    pub fn pow(&self, mut exp: u64) -> Rate {
        let mut base = *self;
        let mut ret = if exp % 2 != 0 {
            base
        } else {
            Rate(Self::wad())
        };

        while exp > 0 {
            exp /= 2;
            base *= base;

            if exp % 2 != 0 {
                ret *= base;
            }
        }

        ret
    }
}

impl fmt::Display for Rate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut scaled_val = self.0.to_string();
        if scaled_val.len() <= SCALE {
            scaled_val.insert_str(0, &vec!["0"; SCALE - scaled_val.len()].join(""));
            scaled_val.insert_str(0, "0.");
        } else {
            scaled_val.insert(scaled_val.len() - SCALE, '.');
        }
        f.write_str(&scaled_val)
    }
}

// TODO: assert that `val` doesn't exceed max u64 wad (~1844)
impl From<u64> for Rate {
    fn from(val: u64) -> Self {
        Self(Self::wad() * U128::from(val))
    }
}

impl std::ops::Add for Rate {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl std::ops::Sub for Rate {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl std::ops::Div<u8> for Rate {
    type Output = Self;
    fn div(self, rhs: u8) -> Self::Output {
        Self(self.0 / U128::from(rhs))
    }
}

impl std::ops::Div<u64> for Rate {
    type Output = Self;
    fn div(self, rhs: u64) -> Self::Output {
        Self(self.0 / U128::from(rhs))
    }
}

// TODO: Returned rate could cause overflows
impl std::ops::Div for Rate {
    type Output = Self;
    fn div(self, rhs: Self) -> Self::Output {
        Self(Self::wad() * self.0 / rhs.0)
    }
}

// TODO: Returned rate could cause overflows
impl std::ops::Mul<u8> for Rate {
    type Output = Self;
    fn mul(self, rhs: u8) -> Self::Output {
        Self(self.0 * U128::from(rhs))
    }
}

// TODO: Returned rate could cause overflows
impl std::ops::Mul<u64> for Rate {
    type Output = Self;
    fn mul(self, rhs: u64) -> Self::Output {
        Self(self.0 * U128::from(rhs))
    }
}

// TODO: Returned rate could cause overflows
impl std::ops::Mul for Rate {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self::Output {
        Self(self.0 * rhs.0 / Self::wad())
    }
}

impl std::ops::AddAssign for Rate {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl std::ops::SubAssign for Rate {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl std::ops::DivAssign for Rate {
    fn div_assign(&mut self, rhs: Self) {
        *self = *self / rhs;
    }
}

impl std::ops::MulAssign for Rate {
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}
