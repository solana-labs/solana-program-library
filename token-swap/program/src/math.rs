//! Math utilities for swap calculations

const ONE: u128 = 10_000_000_000;
const PRECISION: u128 = 100;
const ROUNDING_CORRECTION: u128 = ONE / 2;
const APPROXIMATION_ITERATIONS: u32 = 30;

/// Approximate the nth root of a number using a Taylor Series on x ^ n, where
/// 0 < n < 1, expressed as a precise number.
/// Refine the guess for each term, using
///                  x_k ^ n - A
/// x_k+1 = x_k - -----------------
///               n * x_k ^ (n - 1)
/// num = A, root = n, guess = x_k, iterations = k
pub fn nth_root_approximation(precise_num: u128, root: u32, mut precise_guess: u128, iterations: u32) -> Option<u128> {
    if root == 0 {
        return None;
    }
    let precise_root = precise_number(root as u64)?;
    for _ in 0..iterations {
        let raised = precise_pow(precise_guess, root)?;
        let (numerator, negative) = unsigned_difference(raised, precise_num);
        let raised_minus_one = precise_pow(precise_guess, root.checked_sub(1)?)?;
        let denominator = precise_mul(precise_root, raised_minus_one)?;
        let update = precise_div(numerator, denominator)?;
        if update < PRECISION {
            break;
        }
        if negative {
            precise_guess = precise_guess.checked_add(update)?;
        } else {
            precise_guess = precise_guess.checked_sub(update)?;
        }
    }
    Some(precise_guess)
}

/// Converts a u64 to a "precise" number, artifically making it bigger to do
/// more precise calculations without floats
pub fn precise_number(a: u64) -> Option<u128> {
    (a as u128).checked_mul(ONE)
}

/// Converts a u64 to a "precise" number, artifically making it bigger to do
/// more precise calculations without floats
pub fn imprecise_number(a: u128) -> Option<u64> {
    match a.checked_add(ROUNDING_CORRECTION)?.checked_div(ONE) {
        Some(v) => Some(v as u64),
        None => None,
    }
}

/// Floors a precise value
pub fn precise_floor(a: u128) -> Option<u128> {
    a.checked_div(ONE)?.checked_mul(ONE)
}

/// Performs a multiplication on two "precise" integers
pub fn precise_mul(a: u128, b: u128) -> Option<u128> {
    match a.checked_mul(b) {
        Some(v) => v.checked_add(ROUNDING_CORRECTION)?.checked_div(ONE),
        None => {
            if a >= b {
                a.checked_div(ONE)?.checked_mul(b)
            } else {
                b.checked_div(ONE)?.checked_mul(a)
            }
        }
    }
}

/// Performs division on two "precise" unsigned integers
pub fn precise_div(a: u128, b: u128) -> Option<u128> {
    if b == 0 {
        return None;
    }
    match a.checked_mul(ONE) {
        Some(v) => v.checked_add(ROUNDING_CORRECTION)?.checked_div(b),
        None => {
            a.checked_add(ROUNDING_CORRECTION)?.checked_div(b)?.checked_mul(ONE)
        },
    }
}

/// Performs pow on a "precise" unsigned integers
pub fn precise_pow(mut base: u128, exponent: u32) -> Option<u128> {
    let mut result = if exponent.checked_rem_euclid(2)? == 0 {
        ONE
    } else {
        base
    };

    // To minimize the number of operations, we halve the exponent at each
    // iteration and keep squaring the base.
    let mut doubling_exponent = exponent.checked_div(2)?;
    while doubling_exponent != 0 {
        base = precise_mul(base, base)?;

        // For odd exponents, "push" the base onto the result
        if doubling_exponent.checked_rem_euclid(2)? != 0 {
            result = precise_mul(result, base)?;
        }

        // Prepare next iteration
        doubling_exponent = doubling_exponent.checked_div(2)?;
    }
    Some(result)
}

/// Performs a subtraction, returning the result and whether the result is negative
pub fn unsigned_difference(a: u128, b: u128) -> (u128, bool) {
    match a.checked_sub(b) {
        None => (b.checked_sub(a).unwrap(), true),
        Some(v) => (v, false)
    }
}

/// Get the power of a number, where the exponent is expressed as a fraction
/// (numerator / denominator)
pub fn precise_pow_fraction(base: u128, exponent_numerator: u32, exponent_denominator: u32) -> Option<u128> {
    let whole_exponent = exponent_numerator.checked_div(exponent_denominator)?;
    let whole_power = precise_pow(base, whole_exponent)?;
    let remainder_exponent_numerator = exponent_numerator.checked_rem_euclid(exponent_denominator)? as u32;
    if remainder_exponent_numerator == 0 {
        return Some(whole_power);
    }
    println!("base {} numerator {} denominator {}", base, exponent_numerator, exponent_denominator);
    println!("whole {}", whole_power);
    let precise_guess = precise_number(exponent_denominator as u64)?;
    let remainder_power = nth_root_approximation(base, exponent_denominator, precise_guess, APPROXIMATION_ITERATIONS)?;
    println!("remainder root {}", remainder_power);
    let remainder_power = precise_pow(remainder_power, remainder_exponent_numerator)?;
    println!("remainder power {}", remainder_power);
    precise_mul(whole_power, remainder_power)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check_root_approximation(num: u64, root: u32, expected: u64) {
        let precise_num = precise_number(num).unwrap();
        let precise_guess = precise_number(root as u64).unwrap();
        let root = nth_root_approximation(precise_num, root, precise_guess, APPROXIMATION_ITERATIONS).unwrap();
        assert_eq!(imprecise_number(root).unwrap(), expected);
    }

    #[test]
    fn test_root_approximation() {
        // square root
        check_root_approximation(9, 2, 3); // actually 3
        check_root_approximation(101, 2, 10); // actually 10.049875

        // 5th root
        check_root_approximation(500, 5, 3); // actually 3.46572422

        // 10th root
        check_root_approximation(1000000000, 10, 8); // actually 7.943282347242816
    }

    fn check_pow_fraction(base: u64, numerator: u32, denominator: u32, expected: u64) {
        let precise_base = precise_number(base).unwrap();
        let power = precise_pow_fraction(precise_base, numerator, denominator).unwrap();
        assert_eq!(imprecise_number(power).unwrap(), expected);
    }

    #[test]
    fn test_pow_fraction() {
        check_pow_fraction(1, 1, 1, 1);
        check_pow_fraction(2, 2, 1, 4);
        check_pow_fraction(4, 1, 2, 2);
        check_pow_fraction(1_204, 24, 50, 2);
    }
}
