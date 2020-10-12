//! Swap calculations and curve implementations

use crate::math::PreciseNumber;

/// Initial amount of pool tokens for swap contract, hard-coded to something
/// "sensible" given a maximum of u64.
/// Note that on Ethereum, Uniswap uses the geometric mean of all provided
/// input amounts, and Balancer uses 100 * 10 ^ 18.
pub const INITIAL_SWAP_POOL_AMOUNT: u64 = 1_000_000_000;

/// Encodes all results of swapping from a source token to a destination token
pub struct SwapResult {
    /// New amount of source token
    pub new_source_amount: u64,
    /// New amount of destination token
    pub new_destination_amount: u64,
    /// Amount of destination token swapped
    pub amount_swapped: u64,
}

impl SwapResult {
    /// SwapResult for swap from one currency into another, given pool information
    /// and fee.
    ///
    /// A_o = B_o * (1 - ( B_i / (B_i + A_i - fee) ) ^ (W_i / W_o))
    ///
    /// A_o = amount out, A_i = source amount in,
    /// B_o = destination amount, B_i = source amount,
    /// W_i = weight source token, W_o = weight destination token
    pub fn swap_to(
        source_amount: u64,
        swap_source_amount: u64,
        swap_destination_amount: u64,
        source_weight: u8,
        destination_weight: u8,
        fee_numerator: u64,
        fee_denominator: u64,
    ) -> Option<SwapResult> {
        // debit the fee to calculate the amount swapped
        let mut fee = source_amount
            .checked_mul(fee_numerator)?
            .checked_div(fee_denominator)?;
        if fee == 0 {
            fee = 1; // minimum fee of one token
        }
        let new_source_amount_less_fee = swap_source_amount
            .checked_add(source_amount)?
            .checked_sub(fee)?;

        // Calculate the base of the exponential term
        let precise_swap_source_amount = PreciseNumber::new(swap_source_amount)?;
        let new_source_amount_less_fee = PreciseNumber::new(new_source_amount_less_fee)?;
        let base = precise_swap_source_amount.checked_div(&new_source_amount_less_fee)?;

        // Calculate the exponent
        let exponent_numerator = PreciseNumber::new(source_weight as u64)?;
        let exponent_denominator = PreciseNumber::new(destination_weight as u64)?;
        let exponent = exponent_numerator.checked_div(&exponent_denominator)?;

        // Calculate the change factor
        let factor = PreciseNumber::new(1)?.checked_sub(&base.checked_pow_fraction(&exponent)?)?;

        // Calculate the output values
        let precise_swap_destination_amount = PreciseNumber::new(swap_destination_amount)?;
        let amount_swapped = precise_swap_destination_amount
            .checked_mul(&factor)?
            .to_imprecise()?;
        let new_destination_amount = swap_destination_amount.checked_sub(amount_swapped)?;

        // actually add the whole amount coming in
        let new_source_amount = swap_source_amount.checked_add(source_amount)?;
        Some(SwapResult {
            new_source_amount,
            new_destination_amount,
            amount_swapped,
        })
    }
}

fn map_zero_to_none(x: u64) -> Option<u64> {
    if x == 0 {
        None
    } else {
        Some(x)
    }
}

/// The Uniswap invariant calculator.
pub struct ConstantProduct {
    /// Token A
    pub token_a: u64,
    /// Token B
    pub token_b: u64,
    /// Weight of token A
    pub weight_a: u8,
    /// Weight of token B
    pub weight_b: u8,
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
            self.weight_a,
            self.weight_b,
            self.fee_numerator,
            self.fee_denominator,
        )?;
        self.token_a = result.new_source_amount;
        self.token_b = result.new_destination_amount;
        map_zero_to_none(result.amount_swapped)
    }

    /// Swap token b to a
    pub fn swap_b_to_a(&mut self, token_b: u64) -> Option<u64> {
        let result = SwapResult::swap_to(
            token_b,
            self.token_b,
            self.token_a,
            self.weight_b,
            self.weight_a,
            self.fee_numerator,
            self.fee_denominator,
        )?;
        self.token_b = result.new_source_amount;
        self.token_a = result.new_destination_amount;
        map_zero_to_none(result.amount_swapped)
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
    pub fn new_pool(token_a: u64, token_b: u64) -> Self {
        let supply = INITIAL_SWAP_POOL_AMOUNT;
        Self {
            supply,
            token_a,
            token_b,
        }
    }

    /// A tokens for pool tokens, returns None if output is less than 0
    pub fn token_a_rate(&self, pool_tokens: u64) -> Option<u64> {
        pool_tokens
            .checked_mul(self.token_a)?
            .checked_div(self.supply)
            .and_then(map_zero_to_none)
    }

    /// B tokens for pool tokens, returns None is output is less than 0
    pub fn token_b_rate(&self, pool_tokens: u64) -> Option<u64> {
        pool_tokens
            .checked_mul(self.token_b)?
            .checked_div(self.supply)
            .and_then(map_zero_to_none)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_pool_amount() {
        let token_converter = PoolTokenConverter::new_pool(1, 5);
        assert_eq!(token_converter.supply, INITIAL_SWAP_POOL_AMOUNT);
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

    #[test]
    fn swap_calculation() {
        // calculation on https://github.com/solana-labs/solana-program-library/issues/341
        let swap_source_amount = 1_000;
        let swap_destination_amount = 50_000;
        let source_weight = 1;
        let destination_weight = 1;
        let fee_numerator = 1;
        let fee_denominator = 100;
        let source_amount = 100;
        let result = SwapResult::swap_to(
            source_amount,
            swap_source_amount,
            swap_destination_amount,
            source_weight,
            destination_weight,
            fee_numerator,
            fee_denominator,
        )
        .unwrap();
        assert_eq!(result.new_source_amount, 1_100);
        assert_eq!(result.amount_swapped, 4_504);
        assert_eq!(result.new_destination_amount, 45_496);
    }

    #[test]
    fn weighted_swap_calculation() {
        // calculation on https://github.com/solana-labs/solana-program-library/pull/574
        // 5000 * (1 - ( 5000 / (5000 + 100) ) ^ (1 / 9)) = 10.989
        let swap_source_amount = 5_000_000;
        let swap_destination_amount = 5_000_000;
        let source_weight = 1;
        let destination_weight = 9;
        let fee_numerator = 0;
        let fee_denominator = 100;
        let source_amount = 100_000;
        let result = SwapResult::swap_to(
            source_amount,
            swap_source_amount,
            swap_destination_amount,
            source_weight,
            destination_weight,
            fee_numerator,
            fee_denominator,
        )
        .unwrap();
        assert_eq!(result.amount_swapped, 10_989);
        assert_eq!(result.new_source_amount, 5_100_000);
        assert_eq!(result.new_destination_amount, 4_989_011);
    }
}
