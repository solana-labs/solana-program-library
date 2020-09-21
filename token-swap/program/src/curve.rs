//! Swap calculations and curve implementations

use crate::math;

/// Encodes all results of swapping from a source token to a destination token
pub struct SwapResult {
    /// New amount of source token
    pub new_source: u64,
    /// New amount of destination token
    pub new_destination: u64,
    /// Amount of destination token swapped
    pub amount_swapped: u64,
}

impl SwapResult {
    /// SwapResult for swap from one currency into another, given pool information
    /// and fee
    pub fn swap_to(
        source: u64,
        source_amount: u64,
        dest_amount: u64,
        fee_numerator: u64,
        fee_denominator: u64,
    ) -> Option<SwapResult> {
        let invariant = source_amount.checked_mul(dest_amount)?;
        let new_source = source_amount.checked_add(source)?;
        let new_destination = invariant.checked_div(new_source)?;
        let remove = dest_amount.checked_sub(new_destination)?;
        let fee = remove
            .checked_mul(fee_numerator)?
            .checked_div(fee_denominator)?;
        let new_destination = new_destination.checked_add(fee)?;
        let amount_swapped = remove.checked_sub(fee)?;
        Some(SwapResult {
            new_source,
            new_destination,
            amount_swapped,
        })
    }
}

/// The Uniswap invariant calculator.
pub struct ConstantProduct {
    /// Token A
    pub token_a: u64,
    /// Token B
    pub token_b: u64,
    /// Fee numerator
    pub fee_numerator: u64,
    /// Fee denominator
    pub fee_denominator: u64,
}

impl ConstantProduct {
    /// Swap token a to b
    pub fn swap_a_to_b(&mut self, token_a: u64) -> Option<u64> {
        let result = SwapResult::swap_to(
            token_a,
            self.token_a,
            self.token_b,
            self.fee_numerator,
            self.fee_denominator,
        )?;
        self.token_a = result.new_source;
        self.token_b = result.new_destination;
        Some(result.amount_swapped)
    }

    /// Swap token b to a
    pub fn swap_b_to_a(&mut self, token_b: u64) -> Option<u64> {
        let result = SwapResult::swap_to(
            token_b,
            self.token_b,
            self.token_a,
            self.fee_numerator,
            self.fee_denominator,
        )?;
        self.token_b = result.new_source;
        self.token_a = result.new_destination;
        Some(result.amount_swapped)
    }
}

/// Conversions for pool tokens, how much to deposit / withdraw, along with
/// proper initialization
pub struct PoolTokenConverter {
    /// Total supply
    pub supply: u64,
    /// Token A amount
    pub token_a: u64,
    /// Token B amount
    pub token_b: u64,
}

impl PoolTokenConverter {
    /// Create a converter based on existing market information
    pub fn new_existing(supply: u64, token_a: u64, token_b: u64) -> Self {
        Self {
            supply,
            token_a,
            token_b,
        }
    }

    /// Create a converter for a new pool token, no supply present yet.
    /// According to Uniswap, the geometric mean protects the pool creator
    /// in case the initial ratio is off the market.
    pub fn new_pool(token_a: u64, token_b: u64) -> Option<Self> {
        let supply = math::geometric_mean(&[token_a, token_b])?;
        Some(Self {
            supply,
            token_a,
            token_b,
        })
    }

    /// A tokens for pool tokens
    pub fn token_a_rate(&self, pool_tokens: u64) -> Option<u64> {
        pool_tokens
            .checked_mul(self.token_a)?
            .checked_div(self.supply)
    }

    /// B tokens for pool tokens
    pub fn token_b_rate(&self, pool_tokens: u64) -> Option<u64> {
        pool_tokens
            .checked_mul(self.token_b)?
            .checked_div(self.supply)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn check_initial_pool_amount(token_a: u64, token_b: u64, expected: Option<u64>) {
        match PoolTokenConverter::new_pool(token_a, token_b) {
            None => assert_eq!(expected, None),
            Some(converter) => assert_eq!(converter.supply, expected.unwrap()),
        };
    }

    #[test]
    fn initial_pool_amount() {
        check_initial_pool_amount(1, 4, Some(2));
        check_initial_pool_amount(1, 5, Some(2));
        check_initial_pool_amount(100, 1000, Some(316));
        check_initial_pool_amount(u64::MAX, u64::MAX, None);
        check_initial_pool_amount(u64::MIN, u64::MAX, Some(0));
    }

    fn check_pool_token_a_rate(
        token_a: u64,
        token_b: u64,
        deposit: u64,
        supply: u64,
        expected: Option<u64>,
    ) {
        let calculator = PoolTokenConverter::new_existing(supply, token_a, token_b);
        assert_eq!(calculator.token_a_rate(deposit), expected);
    }

    #[test]
    fn issued_tokens() {
        check_pool_token_a_rate(2, 50, 5, 10, Some(1));
        check_pool_token_a_rate(10, 10, 5, 10, Some(5));
        check_pool_token_a_rate(5, 100, 5, 10, Some(2));
        check_pool_token_a_rate(5, u64::MAX, 5, 10, Some(2));
        check_pool_token_a_rate(u64::MAX, u64::MAX, 5, 10, None);
    }
}
