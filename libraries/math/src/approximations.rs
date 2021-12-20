//! Approximation calculations

use {
    num_traits::{CheckedShl, CheckedShr, PrimInt},
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
}
