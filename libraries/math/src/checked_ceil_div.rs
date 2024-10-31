//! Defines performing checked ceiling division for different types

use crate::uint::U256;

/// Perform a division that does not truncate value from either side, returning
/// the (quotient, divisor) as a tuple
///
/// When dividing integers, we are often left with a remainder, which can
/// cause information to be lost.  By checking for a remainder, adjusting
/// the quotient, and recalculating the divisor, this provides the most fair
/// calculation.
///
/// For example, 400 / 32 = 12, with a remainder cutting off 0.5 of amount.
/// If we simply ceiling the quotient to 13, then we're saying 400 / 32 = 13,
/// which also cuts off value.  To improve this result, we calculate the other
/// way around and again check for a remainder: 400 / 13 = 30, with a remainder
/// of 0.77, and we ceiling that value again.  This gives us a final calculation
/// of 400 / 31 = 13, which provides a ceiling calculation without cutting off
/// more value than needed.
///
/// This calculation fails if the divisor is larger than the dividend, to avoid
/// having a result like: 1 / 1000 = 1.
pub trait CheckedCeilDiv: Sized {
    /// Perform ceiling division
    fn checked_ceil_div(&self, rhs: Self) -> Option<(Self, Self)>;
}

impl CheckedCeilDiv for u128 {
    fn checked_ceil_div(&self, mut rhs: Self) -> Option<(Self, Self)> {
        let mut quotient = self.checked_div(rhs)?;
        // Avoid dividing a small number by a big one and returning 1, and instead
        // fail.
        if quotient == 0 {
            return None;
        }

        // Ceiling the destination amount if there's any remainder, which will
        // almost always be the case.
        let remainder = self.checked_rem(rhs)?;
        if remainder > 0 {
            quotient = quotient.checked_add(1)?;
            // calculate the minimum amount needed to get the dividend amount to
            // avoid truncating too much
            rhs = self.checked_div(quotient)?;
            let remainder = self.checked_rem(quotient)?;
            if remainder > 0 {
                rhs = rhs.checked_add(1)?;
            }
        }
        Some((quotient, rhs))
    }
}

impl CheckedCeilDiv for U256 {
    fn checked_ceil_div(&self, mut rhs: Self) -> Option<(Self, Self)> {
        let mut quotient = self.checked_div(rhs)?;
        let zero = U256::from(0);
        let one = U256::from(1);
        // Avoid dividing a small number by a big one and returning 1, and instead
        // fail.
        if quotient == zero {
            return None;
        }

        // Ceiling the destination amount if there's any remainder, which will
        // almost always be the case.
        let remainder = self.checked_rem(rhs)?;
        if remainder > zero {
            quotient = quotient.checked_add(one)?;
            // calculate the minimum amount needed to get the dividend amount to
            // avoid truncating too much
            rhs = self.checked_div(quotient)?;
            let remainder = self.checked_rem(quotient)?;
            if remainder > zero {
                rhs = rhs.checked_add(one)?;
            }
        }
        Some((quotient, rhs))
    }
}
