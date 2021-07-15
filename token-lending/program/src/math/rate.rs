//! Math for preserving precision of ratios and percentages.
//!
//! Usages and their ranges include:
//!   - Collateral exchange ratio <= 5.0
//!   - Loan to value ratio <= 0.9
//!   - Max borrow rate <= 2.56
//!   - Percentages <= 1.0
//!
//! Rates are internally scaled by a WAD (10^18) to preserve
//! precision up to 18 decimal places. Rates are sized to support
//! both serialization and precise math for the full range of
//! unsigned 8-bit integers. The underlying representation is a
//! u128 rather than u192 to reduce compute cost while losing
//! support for arithmetic operations at the high end of u8 range.

#![allow(clippy::assign_op_pattern)]
#![allow(clippy::ptr_offset_with_cast)]
#![allow(clippy::reversed_empty_ranges)]
#![allow(clippy::manual_range_contains)]

use crate::{
    error::LendingError,
    math::{common::*, decimal::Decimal},
};
use solana_program::program_error::ProgramError;
use std::{convert::TryFrom, fmt};
use uint::construct_uint;

// U128 with 128 bits consisting of 2 x 64-bit words
construct_uint! {
    pub struct U128(2);
}

/// Small decimal values, precise to 18 digits
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

    // OPTIMIZE: use const slice when fixed in BPF toolchain
    fn wad() -> U128 {
        U128::from(WAD)
    }

    /// Create scaled decimal from percent value
    pub fn from_percent(percent: u8) -> Self {
        Self(U128::from(percent as u64 * PERCENT_SCALER))
    }

    /// Return raw scaled value
    #[allow(clippy::wrong_self_convention)]
    pub fn to_scaled_val(&self) -> u128 {
        self.0.as_u128()
    }

    /// Create decimal from scaled value
    pub fn from_scaled_val(scaled_val: u64) -> Self {
        Self(U128::from(scaled_val))
    }

    /// Calculates base^exp
    pub fn try_pow(&self, mut exp: u64) -> Result<Rate, ProgramError> {
        let mut base = *self;
        let mut ret = if exp % 2 != 0 {
            base
        } else {
            Rate(Self::wad())
        };

        while exp > 0 {
            exp /= 2;
            base = base.try_mul(base)?;

            if exp % 2 != 0 {
                ret = ret.try_mul(base)?;
            }
        }

        Ok(ret)
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

impl TryFrom<Decimal> for Rate {
    type Error = ProgramError;
    fn try_from(decimal: Decimal) -> Result<Self, Self::Error> {
        Ok(Self(U128::from(decimal.to_scaled_val()?)))
    }
}

impl TryAdd for Rate {
    fn try_add(self, rhs: Self) -> Result<Self, ProgramError> {
        Ok(Self(
            self.0
                .checked_add(rhs.0)
                .ok_or(LendingError::MathOverflow)?,
        ))
    }
}

impl TrySub for Rate {
    fn try_sub(self, rhs: Self) -> Result<Self, ProgramError> {
        Ok(Self(
            self.0
                .checked_sub(rhs.0)
                .ok_or(LendingError::MathOverflow)?,
        ))
    }
}

impl TryDiv<u64> for Rate {
    fn try_div(self, rhs: u64) -> Result<Self, ProgramError> {
        Ok(Self(
            self.0
                .checked_div(U128::from(rhs))
                .ok_or(LendingError::MathOverflow)?,
        ))
    }
}

impl TryDiv<Rate> for Rate {
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

impl TryMul<u64> for Rate {
    fn try_mul(self, rhs: u64) -> Result<Self, ProgramError> {
        Ok(Self(
            self.0
                .checked_mul(U128::from(rhs))
                .ok_or(LendingError::MathOverflow)?,
        ))
    }
}

impl TryMul<Rate> for Rate {
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
    fn checked_pow() {
        assert_eq!(Rate::one(), Rate::one().try_pow(u64::MAX).unwrap());
    }
}
