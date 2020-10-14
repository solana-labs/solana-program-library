//! Swap calculations and curve implementations

/// Initial amount of pool tokens for swap contract, hard-coded to something
/// "sensible" given a maximum of u64.
/// Note that on Ethereum, Uniswap uses the geometric mean of all provided
/// input amounts, and Balancer uses 100 * 10 ^ 18.
pub const INITIAL_SWAP_POOL_AMOUNT: u64 = 1_000_000_000;

/// Curve types supported by the token-swap program.
#[repr(C)]
#[derive(Clone, Debug, PartialEq)]
pub enum SwapCurveType {
    /// Uniswap-style constant product curve, invariant = token_a_amount * token_b_amount
    ConstantProduct,
    /// Flat line, always providing 1:1 from one token to another
    Flat,
}

impl SwapCurveType {
    /// Create a swap curve corresponding to the enum provided
    pub fn create_swap_curve(&self, swap_source_amount: u64, swap_destination_amount: u64, fee_numerator: u64, fee_denominator: u64) -> Box<dyn SwapCurve> {
        match self {
            SwapCurveType::ConstantProduct => Box::new(
                ConstantProductCurve {
                    swap_source_amount,
                    swap_destination_amount,
                    fee_numerator,
                    fee_denominator,
                }),
            SwapCurveType::Flat => Box::new(FlatCurve {
                swap_source_amount,
                swap_destination_amount,
                fee_numerator,
                fee_denominator }),
        }
    }
}

/// Trait representing operations required on a swap curve
pub trait SwapCurve {
    /// Calculate how much destination token will be provided given an amount
    /// of source token.
    fn swap(&self, source_amount: u64) -> Option<SwapResult>;
}

/// Encodes all results of swapping from a source token to a destination token
pub struct SwapResult {
    /// New amount of source token
    pub new_source_amount: u64,
    /// New amount of destination token
    pub new_destination_amount: u64,
    /// Amount of destination token swapped
    pub amount_swapped: u64,
}

/// Helper function for mapping to SwapError::CalculationFailure
fn map_zero_to_none(x: u64) -> Option<u64> {
    if x == 0 {
        None
    } else {
        Some(x)
    }
}

/// Simple constant 1:1 swap curve, example of different swap curve implementations
pub struct FlatCurve {
    /// Amount of source token
    pub swap_source_amount: u64,
    /// Amount of destination token
    pub swap_destination_amount: u64,
    /// Fee numerator
    pub fee_numerator: u64,
    /// Fee denominator
    pub fee_denominator: u64,
}

impl SwapCurve for FlatCurve {
    fn swap(&self, source_amount: u64) -> Option<SwapResult> {
        // debit the fee to calculate the amount swapped
        let mut fee = source_amount
            .checked_mul(self.fee_numerator)?
            .checked_div(self.fee_denominator)?;
        if fee == 0 {
            fee = 1; // minimum fee of one token
        }

        let amount_swapped = source_amount.checked_sub(fee)?;
        let new_destination_amount = self.swap_destination_amount.checked_sub(amount_swapped)?;

        // actually add the whole amount coming in
        let new_source_amount = self.swap_source_amount.checked_add(source_amount)?;
        Some(SwapResult {
            new_source_amount,
            new_destination_amount,
            amount_swapped,
        })
    }
}

/// The Uniswap invariant calculator.
pub struct ConstantProductCurve {
    /// Amount of source token
    pub swap_source_amount: u64,
    /// Amount of destination token
    pub swap_destination_amount: u64,
    /// Fee numerator
    pub fee_numerator: u64,
    /// Fee denominator
    pub fee_denominator: u64,
}

impl SwapCurve for ConstantProductCurve {
    fn swap(&self, source_amount: u64) -> Option<SwapResult> {
        let invariant = self.swap_source_amount.checked_mul(self.swap_destination_amount)?;

        // debit the fee to calculate the amount swapped
        let mut fee = source_amount
            .checked_mul(self.fee_numerator)?
            .checked_div(self.fee_denominator)?;
        if fee == 0 {
            fee = 1; // minimum fee of one token
        }
        let new_source_amount_less_fee = self.swap_source_amount
            .checked_add(source_amount)?
            .checked_sub(fee)?;
        let new_destination_amount = invariant.checked_div(new_source_amount_less_fee)?;
        let amount_swapped = map_zero_to_none(self.swap_destination_amount.checked_sub(new_destination_amount)?)?;

        // actually add the whole amount coming in
        let new_source_amount = self.swap_source_amount.checked_add(source_amount)?;
        Some(SwapResult {
            new_source_amount,
            new_destination_amount,
            amount_swapped,
        })
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
    fn constant_product_swap_calculation() {
        // calculation on https://github.com/solana-labs/solana-program-library/issues/341
        let swap_source_amount: u64 = 1000;
        let swap_destination_amount: u64 = 50000;
        let fee_numerator: u64 = 1;
        let fee_denominator: u64 = 100;
        let source_amount: u64 = 100;
        let curve = ConstantProductCurve {
            swap_source_amount,
            swap_destination_amount,
            fee_numerator,
            fee_denominator,
        };
        let result = curve.swap(source_amount).unwrap();
        assert_eq!(result.new_source_amount, 1100);
        assert_eq!(result.amount_swapped, 4505);
        assert_eq!(result.new_destination_amount, 45495);
    }

    #[test]
    fn flat_swap_calculation() {
        let swap_source_amount: u64 = 1000;
        let swap_destination_amount: u64 = 50000;
        let fee_numerator: u64 = 1;
        let fee_denominator: u64 = 100;
        let source_amount: u64 = 100;
        let curve = FlatCurve {
            swap_source_amount,
            swap_destination_amount,
            fee_numerator,
            fee_denominator,
        };
        let result = curve.swap(source_amount).unwrap();
        let amount_swapped = 99;
        assert_eq!(result.new_source_amount, 1100);
        assert_eq!(result.amount_swapped, amount_swapped);
        assert_eq!(result.new_destination_amount, swap_destination_amount - amount_swapped);
    }
}
