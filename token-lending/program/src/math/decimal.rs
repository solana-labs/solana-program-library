//! Math for preserving precision

#![allow(clippy::assign_op_pattern)]
#![allow(clippy::ptr_offset_with_cast)]
#![allow(clippy::manual_range_contains)]

use crate::math::common::*;
use crate::math::Rate;
use std::fmt;
use uint::construct_uint;

// U192 with 192 bits consisting of 3 x 64-bit words
construct_uint! {
    pub struct U192(3);
}

/// Large decimal value precise to 18 digits
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd, Eq, Ord)]
pub struct Decimal(pub U192);

impl Decimal {
    /// One
    pub fn one() -> Self {
        Self(Self::wad())
    }

    /// Zero
    pub fn zero() -> Self {
        Self(U192::zero())
    }

    // TODO: use const slices when fixed
    fn wad() -> U192 {
        U192::from(WAD)
    }

    // TODO: use const slices when fixed
    fn half_wad() -> U192 {
        U192::from(HALF_WAD)
    }

    /// Create scaled decimal from percent value
    pub fn from_percent(percent: u8) -> Self {
        Self(U192::from(percent as u64 * PERCENT_SCALER))
    }

    /// Create scaled decimal from value and scale
    pub fn new(val: u64, scale: usize) -> Self {
        assert!(scale <= SCALE);
        Self(Self::wad() / U192::exp10(scale) * U192::from(val))
    }

    /// Convert to lower precision rate
    pub fn as_rate(self) -> Rate {
        Rate::from_scaled_val(self.0.as_u64() as u128)
    }

    /// Return raw scaled value
    pub fn to_scaled_val(&self) -> u128 {
        self.0.as_u128()
    }

    /// Create decimal from scaled value
    pub fn from_scaled_val(scaled_val: u128) -> Self {
        Self(U192::from(scaled_val))
    }

    /// Round scaled decimal to u64
    pub fn round_u64(&self) -> u64 {
        ((Self::half_wad() + self.0) / Self::wad()).as_u64()
    }
}

impl fmt::Display for Decimal {
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

impl From<u64> for Decimal {
    fn from(val: u64) -> Self {
        Self(Self::wad() * U192::from(val))
    }
}

impl From<u128> for Decimal {
    fn from(val: u128) -> Self {
        Self(Self::wad() * U192::from(val))
    }
}

impl From<Rate> for Decimal {
    fn from(val: Rate) -> Self {
        Self(U192::from(val.to_scaled_val()))
    }
}

impl std::ops::Add for Decimal {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl std::ops::Sub for Decimal {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl std::ops::Div<u64> for Decimal {
    type Output = Self;
    fn div(self, rhs: u64) -> Self::Output {
        Self(self.0 / U192::from(rhs))
    }
}

impl std::ops::Div<Rate> for Decimal {
    type Output = Self;
    fn div(self, rhs: Rate) -> Self::Output {
        self / Self::from(rhs)
    }
}

impl std::ops::Div for Decimal {
    type Output = Self;
    fn div(self, rhs: Self) -> Self::Output {
        Self(Self::wad() * self.0 / rhs.0)
    }
}

impl std::ops::Mul<u64> for Decimal {
    type Output = Self;
    fn mul(self, rhs: u64) -> Self::Output {
        Self(self.0 * U192::from(rhs))
    }
}

impl std::ops::Mul<Rate> for Decimal {
    type Output = Self;
    fn mul(self, rhs: Rate) -> Self::Output {
        Self(self.0 * U192::from(rhs.to_scaled_val()) / Self::wad())
    }
}

impl std::ops::AddAssign for Decimal {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl std::ops::SubAssign for Decimal {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl std::ops::DivAssign for Decimal {
    fn div_assign(&mut self, rhs: Self) {
        *self = *self / rhs;
    }
}

impl std::ops::MulAssign<Rate> for Decimal {
    fn mul_assign(&mut self, rhs: Rate) {
        *self = *self * rhs;
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_scaler() {
        assert_eq!(U192::exp10(SCALE), Decimal::wad());
    }
}
