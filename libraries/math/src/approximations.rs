//! Approximation calculations

use crate::precise_number::PreciseNumber;
use {
    num_traits::{CheckedShl, CheckedShr, Float, PrimInt},
    std::cmp::Ordering,
};

/// Calculate square root of the given number
///
/// Code lovingly adapted from the excellent work at:
///
/// <https://github.com/derekdreery/integer-sqrt-rs>
///
/// The algorithm is based on the implementation in:
///
/// <https://en.wikipedia.org/wiki/Methods_of_computing_square_roots#Binary_numeral_system_(base_2)>
pub fn sqrt<T: PrimInt + CheckedShl + CheckedShr>(radicand: T) -> Option<T> {
    match radicand.cmp(&T::zero()) {
        Ordering::Less => return None,             // fail for less than 0
        Ordering::Equal => return Some(T::zero()), // do nothing for 0
        _ => {}
    }

    // Compute bit, the largest power of 4 <= n
    let max_shift: u32 = T::zero().leading_zeros() - 1;
    let shift: u32 = (max_shift - radicand.leading_zeros()) & !1;
    let mut bit = T::one().checked_shl(shift)?;

    let mut n = radicand;
    let mut result = T::zero();
    while bit != T::zero() {
        let result_with_bit = result.checked_add(&bit)?;
        if n >= result_with_bit {
            n = n.checked_sub(&result_with_bit)?;
            result = result.checked_shr(1)?.checked_add(&bit)?;
        } else {
            result = result.checked_shr(1)?;
        }
        bit = bit.checked_shr(2)?;
    }
    Some(result)
}

/// Calculate approximate natural log of a number
/// using Taylor series of Log_e(x)
///     
/// Ideas from https://math.stackexchange.com/a/977836
///
/// $$ ln(x) = (n-1)*ln(10) + 2 * \sum{ y^{2k+1} \over 2k+1 } $$
///
/// where x = A * 10^(n-1) such that 0 <= A < 10
/// and y = (A-1)/(A+1)
pub fn ln<T: Float + Clone>(input: T) -> Option<PreciseNumber> {
    if input.le(&T::zero()) {
        return None; // fail for less than or equal to 0
    }

    // number of digits of input before decimal
    let mut n = 1;
    let mut mantissa = input.to_f64().unwrap();
    while mantissa.floor() > 10_f64 {
        mantissa /= 10_f64;
        n += 1;
    }

    // y = (A-1)/(A+1)
    let y = PreciseNumber::from((mantissa - 1_f64) / (mantissa + 1_f64))?;
    let sum = PreciseNumber::new(0)?;
    
    let pow = PreciseNumber::new(1)?;
    let two = PreciseNumber::new(2)?;
    for _ in 0..30 {
        sum.checked_add(
            &y.checked_pow(pow.to_imprecise()?)?
                .checked_div(&pow)?
                .checked_mul(&two)?,
        );
        pow.checked_add(&two);
    }

    sum.checked_add(&PreciseNumber::new(
        (std::f64::consts::LN_10 * (n - 1) as f64) as u128,
    )?)
}

/// Calculate the normal cdf of the given number
///
/// The approximation is accurate to 3 digits
///
/// Code lovingly adapted from the excellent work at:
///
/// <https://www.hrpub.org/download/20140305/MS7-13401470.pdf>
///
/// The algorithm is based on the implementation in the paper above.
#[inline(never)]
pub fn f32_normal_cdf(argument: f32) -> f32 {
    const PI: f32 = std::f32::consts::PI;

    let mod_argument = if argument < 0.0 {
        -1.0 * argument
    } else {
        argument
    };
    let tabulation_numerator: f32 =
        (1.0 / (1.0 * (2.0 * PI).sqrt())) * (-1.0 * (mod_argument * mod_argument) / 2.0).exp();
    let tabulation_denominator: f32 =
        0.226 + 0.64 * mod_argument + 0.33 * (mod_argument * mod_argument + 3.0).sqrt();
    let y: f32 = 1.0 - tabulation_numerator / tabulation_denominator;
    if argument < 0.0 {
        1.0 - y
    } else {
        y
    }
}

#[cfg(test)]
mod tests {
    use {super::*, proptest::prelude::*};

    fn check_square_root(radicand: u128) {
        let root = sqrt(radicand).unwrap();
        let lower_bound = root.saturating_sub(1).checked_pow(2).unwrap();
        let upper_bound = root.checked_add(1).unwrap().checked_pow(2).unwrap();
        assert!(radicand as u128 <= upper_bound);
        assert!(radicand as u128 >= lower_bound);
    }

    #[test]
    fn test_square_root_min_max() {
        let test_roots = [0, u64::MAX];
        for i in test_roots.iter() {
            check_square_root(*i as u128);
        }
    }

    proptest! {
        #[test]
        fn test_square_root(a in 0..u64::MAX) {
            check_square_root(a as u128);
        }
    }

    fn check_normal_cdf_f32(argument: f32) {
        let result = f32_normal_cdf(argument);
        let check_result = 0.5 * (1.0 + libm::erff(argument / std::f32::consts::SQRT_2));
        let abs_difference: f32 = (result - check_result).abs();
        assert!(abs_difference <= 0.000_2);
    }

    #[test]
    fn test_normal_cdf_f32_min_max() {
        let test_arguments: [f32; 2] = [f32::MIN, f32::MAX];
        for i in test_arguments.iter() {
            check_normal_cdf_f32(*i as f32)
        }
    }

    proptest! {
        #[test]
        fn test_normal_cdf(a in -1000..1000) {
            check_normal_cdf_f32((a as f32)*0.005);
        }
    }

    fn check_ln(input: f32) {
        if input <= 0_f32 {
            assert!(ln(input).is_none());
            return;
        }

        let approx_log = ln(input).unwrap();
        let std_log = input.ln();
        let (error, _sign) = approx_log.unsigned_sub(&PreciseNumber::new(std_log as u128).unwrap());
        // print!("Act {}, Apr {}, Err {}", std_log, approx_log, error);
        // assert!(error.less_than_or_equal(&PreciseNumber::new(0.000_1 as u128).unwrap()));
    }

    #[test]
    fn test_ln_min_max() {
        let test_arguments: [f32; 2] = [f32::MIN, f32::MAX];
        for i in test_arguments.iter() {
            check_ln(*i as f32)
        }
    }

    proptest! {
        #[test]
        fn test_ln(a in 1..u32::MAX) {
            check_ln(a as f32);
        }
    }
}
