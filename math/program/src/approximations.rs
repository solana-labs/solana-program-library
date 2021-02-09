//! Approximation calculations

/// Perform newtonian root directly on u64
pub fn newtonian_root(radicand: u64, root: u64, mut guess: u64, iterations: u64) -> Option<u64> {
    let zero = 0;
    if radicand == zero {
        return Some(zero);
    }
    if root == zero {
        return None;
    }
    let one = 1;
    let root_minus_one = root.checked_sub(one)?;
    let root_minus_one_whole = root_minus_one as u32;
    let mut last_guess = guess;
    for _ in 0..iterations {
        // x_k+1 = ((n - 1) * x_k + A / (x_k ^ (n - 1))) / n
        let first_term = root_minus_one.checked_mul(guess)?;
        let power = guess.checked_pow(root_minus_one_whole);
        let second_term = match power {
            Some(num) => radicand.checked_div(num)?,
            None => 0,
        };
        guess = first_term.checked_add(second_term)?.checked_div(root)?;
        if last_guess == guess {
            break;
        } else {
            last_guess = guess;
        }
    }
    Some(guess)
}

/// Perform square root
pub fn sqrt(radicand: u64) -> Option<u64> {
    // A good initial guess is the average of the interval that contains the
    // input number.  For all numbers, that will be between 1 and the given number.
    let guess = radicand.checked_div(2)?.checked_add(1)?;
    newtonian_root(radicand, 2, guess, 50)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn check_square_root(radicand: u64) {
        let root = sqrt(radicand).unwrap() as u128;
        let check = root.checked_pow(2).unwrap();
        let lower_bound = root.saturating_sub(1).checked_pow(2).unwrap();
        let upper_bound = root.checked_add(1).unwrap().checked_pow(2).unwrap();
        println!("radicand {} root {} check {}", radicand, root, check);
        assert!(radicand as u128 <= upper_bound);
        assert!(radicand as u128 >= lower_bound);
    }

    #[test]
    fn test_square_root_min_max() {
        let test_roots = [0, u64::MAX];
        for i in test_roots.iter() {
            check_square_root(*i);
        }
    }

    proptest! {
        #[test]
        fn test_square_root(a in 0..u64::MAX) {
            check_square_root(a);
        }
    }
}
