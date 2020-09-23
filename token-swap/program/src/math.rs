//! Math utilities for swap calculations

/// Approximate the nth root of a number using Newton's method
/// https://en.wikipedia.org/wiki/Newton%27s_method
pub fn nth_root_approximation(num: u64, root: u32, mut guess: u64, iterations: u32) -> Option<u64> {
    if root == 0 {
        return None;
    }
    let wide_root = root as u64;
    let root_minus_1 = wide_root.checked_sub(1)?;
    for _ in 0..iterations {
        // x_k+1 = ((n - 1) * x_k + A / (x_k ^ (n - 1))) / n
        let first_term = root_minus_1.checked_mul(guess)?;
        let second_term = num.checked_div(guess.checked_pow(root.checked_sub(1)?)?)?;
        guess = first_term
            .checked_add(second_term)?
            .checked_div(wide_root)?;
    }
    Some(guess)
}

/// Checked sum of all slice elements, needed because `iter().sum()`
/// panics on overflow)
pub fn checked_sum(nums: &[u64]) -> Option<u64> {
    nums.iter().fold(Some(0 as u64), |acc, x| match acc {
        Some(num) => num.checked_add(*x),
        None => None,
    })
}

/// Checked product of all slice elements, needed because `iter().product()`
/// panics on overflow)
pub fn checked_product(nums: &[u64]) -> Option<u64> {
    if nums.is_empty() {
        Some(0)
    } else {
        nums.iter().fold(Some(1 as u64), |acc, x| match acc {
            Some(num) => num.checked_mul(*x),
            None => None,
        })
    }
}

/// Geometric mean of numbers
pub fn geometric_mean(nums: &[u64]) -> Option<u64> {
    let sum = checked_sum(nums)?;
    let product = checked_product(nums)?;
    if product == 0 {
        return Some(product);
    }
    let length = nums.len() as u32;
    // guess is the arithmetic average
    let guess = sum.checked_div(length as u64)?;
    let iterations = length * 2; // arbitrary
    nth_root_approximation(product, length, guess, iterations)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_root_approximation() {
        // square root
        let root = nth_root_approximation(9, 2, 5, 10).unwrap();
        assert_eq!(root, 3); // actually 3
        let root = nth_root_approximation(101, 2, 5, 10).unwrap();
        assert_eq!(root, 10); // actually 10.049875

        // 5th root
        let root = nth_root_approximation(500, 5, 5, 10).unwrap();
        assert_eq!(root, 3); // actually 3.46572422

        // 10th root
        let root = nth_root_approximation(1000000000, 10, 5, 50).unwrap();
        assert_eq!(root, 8); // actually 7.943282347242816
    }

    #[test]
    fn test_geometric_mean() {
        assert_eq!(geometric_mean(&[1, 1, 1]).unwrap(), 1);
        assert_eq!(geometric_mean(&[10, 1000]).unwrap(), 100);
        assert_eq!(geometric_mean(&[0, u64::MAX]).unwrap(), 0);
        assert_eq!(geometric_mean(&[1, u64::MAX]), None);
        assert_eq!(geometric_mean(&[u64::MAX, u64::MAX]), None);
    }

    #[test]
    fn test_checked_product() {
        assert_eq!(checked_product(&[]).unwrap(), 0);
        assert_eq!(checked_product(&[1, 1, 1]).unwrap(), 1);
        assert_eq!(checked_product(&[10, 1000]).unwrap(), 10000);
        assert_eq!(checked_product(&[0, u64::MAX]).unwrap(), 0);
        assert_eq!(checked_product(&[1, u64::MAX]).unwrap(), u64::MAX);
        assert_eq!(checked_product(&[u64::MAX, u64::MAX]), None);
    }

    #[test]
    fn test_checked_sum() {
        assert_eq!(checked_sum(&[]).unwrap(), 0);
        assert_eq!(checked_sum(&[1, 1, 1]).unwrap(), 3);
        assert_eq!(checked_sum(&[10, 1000]).unwrap(), 1010);
        assert_eq!(checked_sum(&[0, u64::MAX]).unwrap(), u64::MAX);
        assert_eq!(checked_sum(&[1, u64::MAX]), None);
        assert_eq!(checked_sum(&[u64::MAX, u64::MAX]), None);
    }
}
