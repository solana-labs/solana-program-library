//! Defines useful math utils

#![allow(clippy::assign_op_pattern)]
#![allow(clippy::ptr_offset_with_cast)]
#![allow(clippy::unknown_clippy_lints)]
#![allow(clippy::manual_range_contains)]

use uint::construct_uint;

construct_uint! {
    pub struct U256(4);
}

impl U256 {
    /// Returns selt to the power of b
    pub fn checked_u8_power(&self, b: u8) -> Option<U256> {
        let mut result = *self;
        for _ in 1..b {
            result = result.checked_mul(*self)?;
        }
        Some(result)
    }

    /// Returns self multiplied by b
    pub fn checked_u8_mul(&self, b: u8) -> Option<U256> {
        let mut result = *self;
        for _ in 1..b {
            result = result.checked_add(*self)?;
        }
        Some(result)
    }

    /// Returns true of values differ not more than by 1
    pub fn almost_equal(&self, b: &U256) -> Option<bool> {
        if self > b {
            Some(self.checked_sub(*b)? <= U256::one())
        } else {
            Some(b.checked_sub(*self)? <= U256::one())
        }
    }
}

/// The representation of the number one as a precise number
pub const ONE: u128 = 10_000_000_000;

/// Maximum weight for token in swap.  This number is meant to stay small to
/// so that it is possible to accurately calculate x^(MAX_WEIGHT / MIN_WEIGHT).
pub const MAX_WEIGHT: u8 = 100;

/// Minimum weight for token in swap
pub const MIN_WEIGHT: u8 = 1;

/// Struct encapsulating a fixed-point number that allows for decimal calculations
#[derive(Clone)]
pub struct PreciseNumber {
    /// Wrapper over the inner value, which is multiplied by ONE
    pub value: U256,
}

impl PreciseNumber {
    /// The number 1 as a PreciseNumber, used for easier calculations.
    const ONE: U256 = U256::from(ONE);
    /// The number 0 as a PreciseNumber, used for easier calculations.
    const ZERO: Self = Self { value: U256::from(0) };

    /// Correction to apply to avoid truncation errors on division.  Since
    /// integer operations will always floor the result, we artifically bump it
    /// up by one half to get the expect result.
    const ROUNDING_CORRECTION: U256 = U256::from(ONE / 2);

    /// Desired precision for the correction factor applied during each
    /// iteration of checked_pow_approximation.  Once the correction factor is
    /// smaller than this number, or we reach the maxmium number of iterations,
    /// the calculation ends.
    const POW_PRECISION: U256 = U256::from(100);

    /// Maximum number iterations to apply on checked_pow_approximation.
    const MAX_APPROXIMATION_ITERATIONS: u128 = 100;

    /// Minimum base allowed when calculating exponents in checked_pow_fraction
    /// and checked_pow_approximation.  This simply avoids 0 as a base.
    const MIN_POW_BASE: U256 = U256::from(1);

    /// Maximum base allowed when calculating exponents in checked_pow_fraction
    /// and checked_pow_approximation.  The calculation use a Taylor Series
    /// approxmation around 1, which converges for bases between 0 and 2.  See
    /// https://en.wikipedia.org/wiki/Binomial_series#Conditions_for_convergence
    /// for more information.
    const MAX_POW_BASE: U256 = U256::from(2 * ONE);

    /// Create a precise number from an imprecise u64, should always succeed
    pub fn new(value: u128) -> Option<Self> {
        let value = U256::from(value).checked_mul(Self::ONE)?;
        Some(Self { value })
    }

    /// Convert a precise number back to u128
    pub fn to_imprecise(&self) -> Option<u128> {
        match self
            .value
            .checked_add(Self::ROUNDING_CORRECTION)?
            .checked_div(Self::ONE)
        {
            Some(v) => Some(v.as_u128()),
            None => None,
        }
    }

    /// Checks that two PreciseNumbers are equal within some tolerance
    pub fn almost_eq(&self, rhs: &Self, precision: U256) -> bool {
        let (difference, _) = self.unsigned_sub(rhs);
        difference.value < precision
    }

    /// Floors a precise value to a precision of ONE
    pub fn floor(&self) -> Option<Self> {
        let value = self.value.checked_div(Self::ONE)?.checked_mul(Self::ONE)?;
        Some(Self { value })
    }

    /// Performs a checked division on two precise numbers
    pub fn checked_div(&self, rhs: &Self) -> Option<Self> {
        if rhs.value == U256::from(0) {
            return None;
        }
        match self.value.checked_mul(Self::ONE) {
            Some(v) => {
                let value = v
                    .checked_add(Self::ROUNDING_CORRECTION)?
                    .checked_div(rhs.value)?;
                Some(Self { value })
            }
            None => {
                let value = self
                    .value
                    .checked_add(Self::ROUNDING_CORRECTION)?
                    .checked_div(rhs.value)?
                    .checked_mul(Self::ONE)?;
                Some(Self { value })
            }
        }
    }

    /// Performs a multiplication on two precise numbers
    pub fn checked_mul(&self, rhs: &Self) -> Option<Self> {
        match self.value.checked_mul(rhs.value) {
            Some(v) => {
                let value = v.checked_add(Self::ROUNDING_CORRECTION)?.checked_div(Self::ONE)?;
                Some(Self { value })
            }
            None => {
                let value = if self.value >= rhs.value {
                    self.value.checked_div(Self::ONE)?.checked_mul(rhs.value)?
                } else {
                    rhs.value.checked_div(Self::ONE)?.checked_mul(self.value)?
                };
                Some(Self { value })
            }
        }
    }

    /// Performs addition of two precise numbers
    pub fn checked_add(&self, rhs: &Self) -> Option<Self> {
        let value = self.value.checked_add(rhs.value)?;
        Some(Self { value })
    }

    /// Subtracts the argument from self
    pub fn checked_sub(&self, rhs: &Self) -> Option<Self> {
        let value = self.value.checked_sub(rhs.value)?;
        Some(Self { value })
    }

    /// Performs a subtraction, returning the result and whether the result is negative
    pub fn unsigned_sub(&self, rhs: &Self) -> (Self, bool) {
        match self.value.checked_sub(rhs.value) {
            None => {
                let value = rhs.value.checked_sub(self.value).unwrap();
                (Self { value }, true)
            }
            Some(value) => (Self { value }, false),
        }
    }

    /// Performs pow on a precise number
    pub fn checked_pow(&self, exponent: u128) -> Option<Self> {
        // For odd powers, start with a multiplication by base since we halve the
        // exponent at the start
        let value = if exponent.checked_rem(2)? == 0 {
            Self::ONE
        } else {
            self.value
        };
        let mut result = Self { value };

        // To minimize the number of operations, we keep squaring the base, and
        // only push to the result on odd exponents, like a binary decomposition
        // of the exponent.
        let mut squared_base = self.clone();
        let mut current_exponent = exponent.checked_div(2)?;
        while current_exponent != 0 {
            squared_base = squared_base.checked_mul(&squared_base)?;

            // For odd exponents, "push" the base onto the value
            if current_exponent.checked_rem(2)? != 0 {
                result = result.checked_mul(&squared_base)?;
            }

            current_exponent = current_exponent.checked_div(2)?;
        }
        Some(result)
    }

    /// Approximate the nth root of a number using a Taylor Series around 1 on
    /// x ^ n, where 0 < n < 1, result is a precise number.
    /// Refine the guess for each term, using:
    ///                                  1                    2
    /// f(x) = f(a) + f'(a) * (x - a) + --- * f''(a) * (x - a)  + ...
    ///                                  2!
    /// For x ^ n, this gives:
    ///  n    n         n-1           1                  n-2        2
    /// x  = a  + n * a    (x - a) + --- * n * (n - 1) a     (x - a)  + ...
    ///                               2!
    ///
    /// More simply, this means refining the term at each iteration with:
    ///
    /// t_k+1 = t_k * (x - a) * (n + 1 - k) / k
    ///
    /// where a = 1, n = power, x = precise_num
    pub fn checked_pow_approximation(&self, exponent: &Self, max_iterations: u128) -> Option<Self> {
        assert!(self.value >= Self::MIN_POW_BASE);
        assert!(self.value <= Self::MAX_POW_BASE);
        let one = Self::new(1)?;
        if exponent.value == U256::from(0) {
            return Some(one);
        }
        let mut precise_guess = one.clone();
        let mut term = precise_guess.clone();
        let (x_minus_a, x_minus_a_negative) = self.unsigned_sub(&precise_guess);
        let exponent_plus_one = exponent.checked_add(&one)?;
        let mut negative = false;
        for k in 1..max_iterations {
            let k = Self::new(k)?;
            let (current_exponent, current_exponent_negative) = exponent_plus_one.unsigned_sub(&k);
            term = term.checked_mul(&current_exponent)?;
            term = term.checked_mul(&x_minus_a)?;
            term = term.checked_div(&k)?;
            if term.value < Self::POW_PRECISION {
                break;
            }
            if x_minus_a_negative {
                negative = !negative;
            }
            if current_exponent_negative {
                negative = !negative;
            }
            if negative {
                precise_guess = precise_guess.checked_sub(&term)?;
            } else {
                precise_guess = precise_guess.checked_add(&term)?;
            }
        }
        Some(precise_guess)
    }

    /// Get the power of a number, where the exponent is expressed as a fraction
    /// (numerator / denominator)
    pub fn checked_pow_fraction(&self, exponent: &Self) -> Option<Self> {
        assert!(self.value >= Self::MIN_POW_BASE);
        assert!(self.value <= Self::MAX_POW_BASE);
        let whole_exponent = exponent.floor()?;
        let precise_whole = self.checked_pow(whole_exponent.to_imprecise()?)?;
        let (remainder_exponent, negative) = exponent.unsigned_sub(&whole_exponent);
        assert!(!negative);
        if remainder_exponent.value == U256::from(0) {
            return Some(precise_whole);
        }
        let precise_remainder = self
            .checked_pow_approximation(&remainder_exponent, Self::MAX_APPROXIMATION_ITERATIONS)?;
        precise_whole.checked_mul(&precise_remainder)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const POW_TEST_PRECISION: U256 = U256::from(5_000_000); // correct to at least 3 decimal places

    fn check_pow_approximation(base: U256, exponent: U256, expected: U256) {
        let base = PreciseNumber { value: base };
        let exponent = PreciseNumber { value: exponent };
        let root = base
            .checked_pow_approximation(&exponent, PreciseNumber::MAX_APPROXIMATION_ITERATIONS)
            .unwrap();
        let expected = PreciseNumber { value: expected };
        assert!(root.almost_eq(&expected, POW_TEST_PRECISION));
    }

    #[test]
    fn test_root_approximation() {
        // square root
        check_pow_approximation(ONE / 4, ONE / 2, ONE / 2); // 1/2
        check_pow_approximation(ONE / 101, ONE / 2, U256::from(995037190)); // 0.099503719020999

        // 5th root
        check_pow_approximation(ONE / 500, ONE * 2 / 5, U256::from(832500000)); // 0.08325

        // 10th root
        check_pow_approximation(ONE / 1000, ONE * 4 / 50, U256::from(5754300000)); // 0.57543
    }

    fn check_pow_fraction(base: U256, exponent: U256, expected: U256) {
        let base = PreciseNumber { value: base };
        let exponent = PreciseNumber { value: exponent };
        let power = base.checked_pow_fraction(&exponent).unwrap();
        let expected = PreciseNumber { value: expected };
        assert!(power.almost_eq(&expected, POW_TEST_PRECISION));
    }

    #[test]
    fn test_pow_fraction() {
        check_pow_fraction(ONE, ONE, ONE);
        check_pow_fraction(ONE * 2, ONE * 2, ONE * 4);
        check_pow_fraction(ONE * 2, ONE * 50 / 3, U256::from(1040319153417880));
        check_pow_fraction(ONE * 2 / 7, ONE * 49 / 4, U256::from(2163));
        check_pow_fraction(ONE * 5000 / 5100, ONE / 9, U256::from(9978021269)); // 0.99780212695
    }
}
