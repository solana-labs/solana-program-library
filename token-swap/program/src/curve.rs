//! Swap calculations and curve implementations

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
    /// and fee
    pub fn swap_to(
        source_amount: u64,
        swap_source_amount: u64,
        swap_destination_amount: u64,
        fee_numerator: u64,
        fee_denominator: u64,
    ) -> Option<SwapResult> {
        let invariant = swap_source_amount.checked_mul(swap_destination_amount)?;
        let new_source_amount = swap_source_amount.checked_add(source_amount)?;
        let new_destination_amount = invariant.checked_div(new_source_amount)?;
        let remove = swap_destination_amount.checked_sub(new_destination_amount)?;
        let fee = remove
            .checked_mul(fee_numerator)?
            .checked_div(fee_denominator)?;
        let new_destination_amount = new_destination_amount.checked_add(fee)?;
        let amount_swapped = remove.checked_sub(fee)?;
        Some(SwapResult {
            new_source_amount,
            new_destination_amount,
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
        self.token_a = result.new_source_amount;
        self.token_b = result.new_destination_amount;
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
        self.token_b = result.new_source_amount;
        self.token_a = result.new_destination_amount;
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
    pub fn new_pool(token_a: u64, token_b: u64) -> Self {
        let supply = INITIAL_SWAP_POOL_AMOUNT;
        Self {
            supply,
            token_a,
            token_b,
        }
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
}
