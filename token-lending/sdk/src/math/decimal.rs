//! Math for preserving precision of token amounts which are limited
//! by the SPL Token program to be at most u64::MAX.
//!
//! Decimals are internally scaled by a WAD (10^18) to preserve
//! precision up to 18 decimal places. Decimals are sized to support
//! both serialization and precise math for the full range of
//! unsigned 64-bit integers. The underlying representation is a
//! u192 rather than u256 to reduce compute cost while losing
//! support for arithmetic operations at the high end of u64 range.

#![allow(clippy::assign_op_pattern)]
#![allow(clippy::ptr_offset_with_cast)]
#![allow(clippy::manual_range_contains)]

use crate::{
    error::LendingError,
    math::{common::*, Rate},
};
use solana_program::program_error::ProgramError;
use std::{convert::TryFrom, fmt};
use uint::construct_uint;

// U192 with 192 bits consisting of 3 x 64-bit words
construct_uint! {
    pub struct U192(3);
}

/// Large decimal values, precise to 18 digits
#[derive(Clone, Copy, Default, PartialEq, PartialOrd, Eq, Ord)]
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

    // OPTIMIZE: use const slice when fixed in BPF toolchain
    fn wad() -> U192 {
        U192::from(WAD)
    }

    // OPTIMIZE: use const slice when fixed in BPF toolchain
    fn half_wad() -> U192 {
        U192::from(HALF_WAD)
    }

    /// Create scaled decimal from percent value
    pub fn from_percent(percent: u8) -> Self {
        Self(U192::from(percent as u64 * PERCENT_SCALER))
    }

    /// Create scaled decimal from bps value
    pub fn from_bps(bps: u64) -> Self {
        Self::from(bps).try_div(10_000).unwrap()
    }

    /// Return raw scaled value if it fits within u128
    #[allow(clippy::wrong_self_convention)]
    pub fn to_scaled_val(&self) -> Result<u128, ProgramError> {
        Ok(u128::try_from(self.0).map_err(|_| LendingError::MathOverflow)?)
    }

    /// Create decimal from scaled value
    pub fn from_scaled_val(scaled_val: u128) -> Self {
        Self(U192::from(scaled_val))
    }

    /// Round scaled decimal to u64
    pub fn try_round_u64(&self) -> Result<u64, ProgramError> {
        let rounded_val = Self::half_wad()
            .checked_add(self.0)
            .ok_or(LendingError::MathOverflow)?
            .checked_div(Self::wad())
            .ok_or(LendingError::MathOverflow)?;
        Ok(u64::try_from(rounded_val).map_err(|_| LendingError::MathOverflow)?)
    }

    /// Ceiling scaled decimal to u64
    pub fn try_ceil_u64(&self) -> Result<u64, ProgramError> {
        let ceil_val = Self::wad()
            .checked_sub(U192::from(1u64))
            .ok_or(LendingError::MathOverflow)?
            .checked_add(self.0)
            .ok_or(LendingError::MathOverflow)?
            .checked_div(Self::wad())
            .ok_or(LendingError::MathOverflow)?;
        Ok(u64::try_from(ceil_val).map_err(|_| LendingError::MathOverflow)?)
    }

    /// Floor scaled decimal to u64
    pub fn try_floor_u64(&self) -> Result<u64, ProgramError> {
        let ceil_val = self
            .0
            .checked_div(Self::wad())
            .ok_or(LendingError::MathOverflow)?;
        Ok(u64::try_from(ceil_val).map_err(|_| LendingError::MathOverflow)?)
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

impl fmt::Debug for Decimal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
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

impl TryAdd for Decimal {
    fn try_add(self, rhs: Self) -> Result<Self, ProgramError> {
        Ok(Self(
            self.0
                .checked_add(rhs.0)
                .ok_or(LendingError::MathOverflow)?,
        ))
    }
}

impl TrySub for Decimal {
    fn try_sub(self, rhs: Self) -> Result<Self, ProgramError> {
        Ok(Self(
            self.0
                .checked_sub(rhs.0)
                .ok_or(LendingError::MathOverflow)?,
        ))
    }
}

impl TryDiv<u64> for Decimal {
    fn try_div(self, rhs: u64) -> Result<Self, ProgramError> {
        Ok(Self(
            self.0
                .checked_div(U192::from(rhs))
                .ok_or(LendingError::MathOverflow)?,
        ))
    }
}

impl TryDiv<Rate> for Decimal {
    fn try_div(self, rhs: Rate) -> Result<Self, ProgramError> {
        self.try_div(Self::from(rhs))
    }
}

impl TryDiv<Decimal> for Decimal {
    fn try_div(self, rhs: Self) -> Result<Self, ProgramError> {
        Ok(Self(
            self.0
                .checked_mul(Self::wad())
                .ok_or(LendingError::MathOverflow)?
                .checked_div(rhs.0)
                .ok_or(LendingError::MathOverflow)?,
        ))
    }
}

impl TryMul<u64> for Decimal {
    fn try_mul(self, rhs: u64) -> Result<Self, ProgramError> {
        Ok(Self(
            self.0
                .checked_mul(U192::from(rhs))
                .ok_or(LendingError::MathOverflow)?,
        ))
    }
}

impl TryMul<Rate> for Decimal {
    fn try_mul(self, rhs: Rate) -> Result<Self, ProgramError> {
        self.try_mul(Self::from(rhs))
    }
}

impl TryMul<Decimal> for Decimal {
    fn try_mul(self, rhs: Self) -> Result<Self, ProgramError> {
        Ok(Self(
            self.0
                .checked_mul(rhs.0)
                .ok_or(LendingError::MathOverflow)?
                .checked_div(Self::wad())
                .ok_or(LendingError::MathOverflow)?,
        ))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_scaler() {
        assert_eq!(U192::exp10(SCALE), Decimal::wad());
    }

    #[test]
    fn test_u192() {
        let one = U192::from(1);
        assert_eq!(one.0, [1u64, 0, 0]);

        let wad = Decimal::wad();
        assert_eq!(wad.0, [WAD, 0, 0]);

        let hundred = Decimal::from(100u64);
        // 2^64 * 5 + 7766279631452241920 = 1e20
        assert_eq!(hundred.0 .0, [7766279631452241920, 5, 0]);
    }

    #[test]
    fn test_from_percent() {
        let left = Decimal::from_percent(20);
        let right = Decimal::from(20u64).try_div(Decimal::from(100u64)).unwrap();

        assert_eq!(left, right);
    }

    #[test]
    fn test_from_bps() {
        let left = Decimal::from_bps(190000);
        assert_eq!(left, Decimal::from(19u64));
    }

    #[test]
    fn test_to_scaled_val() {
        assert_eq!(
            Decimal(U192::from(u128::MAX)).to_scaled_val().unwrap(),
            u128::MAX
        );

        assert_eq!(
            Decimal(U192::from(u128::MAX))
                .try_add(Decimal(U192::from(1)))
                .unwrap()
                .to_scaled_val(),
            Err(ProgramError::from(LendingError::MathOverflow))
        );
    }

    #[test]
    fn test_round_floor_ceil_u64() {
        let mut val = Decimal::one();
        assert_eq!(val.try_round_u64().unwrap(), 1);
        assert_eq!(val.try_floor_u64().unwrap(), 1);
        assert_eq!(val.try_ceil_u64().unwrap(), 1);

        val = val
            .try_add(Decimal::from_scaled_val(HALF_WAD as u128 - 1))
            .unwrap();
        assert_eq!(val.try_round_u64().unwrap(), 1);
        assert_eq!(val.try_floor_u64().unwrap(), 1);
        assert_eq!(val.try_ceil_u64().unwrap(), 2);

        val = val.try_add(Decimal::from_scaled_val(1)).unwrap();
        assert_eq!(val.try_round_u64().unwrap(), 2);
        assert_eq!(val.try_floor_u64().unwrap(), 1);
        assert_eq!(val.try_ceil_u64().unwrap(), 2);
    }

    #[test]
    fn test_display() {
        assert_eq!(Decimal::from(1u64).to_string(), "1.000000000000000000");
        assert_eq!(
            Decimal::from_scaled_val(1u128).to_string(),
            "0.000000000000000001"
        );
    }
}
