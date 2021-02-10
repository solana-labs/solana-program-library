//! Approximation calculations

use {
    num_traits::{CheckedAdd, CheckedDiv, One, Zero},
    std::cmp::Eq,
};

const SQRT_ITERATIONS: u8 = 50;

/// Perform square root
pub fn sqrt<T: CheckedAdd + CheckedDiv + One + Zero + Eq + Copy>(radicand: T) -> Option<T> {
    if radicand == T::zero() {
        return Some(T::zero());
    }
    // A good initial guess is the average of the interval that contains the
    // input number.  For all numbers, that will be between 1 and the given number.
    let one = T::one();
    let two = one.checked_add(&one)?;
    let mut guess = radicand.checked_div(&two)?.checked_add(&one)?;
    let mut last_guess = guess;
    for _ in 0..SQRT_ITERATIONS {
        // x_k+1 = (x_k + radicand / x_k) / 2
        guess = last_guess
            .checked_add(&radicand.checked_div(&last_guess)?)?
            .checked_div(&two)?;
        if last_guess == guess {
            break;
        } else {
            last_guess = guess;
        }
    }
    Some(guess)
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
