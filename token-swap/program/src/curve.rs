//! Swap calculations and curve implementations

use solana_sdk::program_error::ProgramError;
use std::convert::TryFrom;

/// Initial amount of pool tokens for swap contract, hard-coded to something
/// "sensible" given a maximum of u64.
/// Note that on Ethereum, Uniswap uses the geometric mean of all provided
/// input amounts, and Balancer uses 100 * 10 ^ 18.
pub const INITIAL_SWAP_POOL_AMOUNT: u64 = 1_000_000_000;

/// Curve types supported by the token-swap program.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CurveType {
    /// Uniswap-style constant product curve, invariant = token_a_amount * token_b_amount
    ConstantProduct,
    /// Flat line, always providing 1:1 from one token to another
    Flat,
}

impl Default for CurveType {
    fn default() -> Self {
        CurveType::ConstantProduct
    }
}

impl TryFrom<u8> for CurveType {
    type Error = ProgramError;

    fn try_from(curve_type: u8) -> Result<Self, Self::Error> {
        match curve_type {
            0 => Ok(CurveType::ConstantProduct),
            1 => Ok(CurveType::Flat),
            _ => Err(ProgramError::InvalidAccountData),
        }
    }
}

impl CurveType {
    /// Create a swap curve corresponding to the enum
    pub fn swap_curve(
        &self,
        swap_source_amount: u64,
        swap_destination_amount: u64,
        fee_numerator: u64,
        fee_denominator: u64,
    ) -> Box<dyn SwapCurve> {
        match self {
            CurveType::ConstantProduct => Box::new(ConstantProductCurve {
                swap_source_amount,
                swap_destination_amount,
                fee_numerator,
                fee_denominator,
            }),
            CurveType::Flat => Box::new(FlatCurve {
                swap_source_amount,
                swap_destination_amount,
                fee_numerator,
                fee_denominator,
            }),
        }
    }

    /// Create a pool token converter for an existing pool
    pub fn existing_pool_token_converter(&self, pool_tokens: u64) -> Box<dyn PoolTokenConverter> {
        match self {
            CurveType::ConstantProduct => {
                Box::new(RelativePoolTokenConverter::new_existing(pool_tokens))
            }
            CurveType::Flat => Box::new(RelativePoolTokenConverter::new_existing(pool_tokens)),
        }
    }

    /// Create a pool token converter for a new pool
    pub fn new_pool_token_converter(&self) -> Box<dyn PoolTokenConverter> {
        match self {
            CurveType::ConstantProduct => Box::new(RelativePoolTokenConverter::new_pool()),
            CurveType::Flat => Box::new(RelativePoolTokenConverter::new_pool()),
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
        let invariant = self
            .swap_source_amount
            .checked_mul(self.swap_destination_amount)?;

        // debit the fee to calculate the amount swapped
        let mut fee = source_amount
            .checked_mul(self.fee_numerator)?
            .checked_div(self.fee_denominator)?;
        if fee == 0 {
            fee = 1; // minimum fee of one token
        }
        let new_source_amount_less_fee = self
            .swap_source_amount
            .checked_add(source_amount)?
            .checked_sub(fee)?;
        let new_destination_amount = invariant.checked_div(new_source_amount_less_fee)?;
        let amount_swapped = map_zero_to_none(
            self.swap_destination_amount
                .checked_sub(new_destination_amount)?,
        )?;

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
pub trait PoolTokenConverter {
    /// Create a converter based on an existing supply
    fn new_existing(pool_token_supply: u64) -> Self
    where
        Self: Sized;
    /// Create a totally new pool with some default amount
    fn new_pool() -> Self
    where
        Self: Sized;
    /// Get the amount of liquidity tokens for pool tokens given the total amount
    /// of liquidity tokens in the pool
    fn liquidity_tokens(&self, pool_tokens: u64, total_liquidity_tokens: u64) -> Option<u64>;
    /// Get total pool token supply
    fn supply(&self) -> u64;
}

/// Balancer-style pool token converter, which initializes with a hard-coded
/// amount, then converts pool tokens relative to each amount of liquidity
/// token.
pub struct RelativePoolTokenConverter {
    /// Total pool token supply
    pub pool_token_supply: u64,
}

impl PoolTokenConverter for RelativePoolTokenConverter {
    /// Create a converter based on existing market information
    fn new_existing(pool_token_supply: u64) -> Self {
        Self { pool_token_supply }
    }

    /// Get total pool token supply
    fn supply(&self) -> u64 {
        self.pool_token_supply
    }

    /// Create a converter for a new pool token, no supply present yet.
    /// According to Uniswap, the geometric mean protects the pool creator
    /// in case the initial ratio is off the market.
    fn new_pool() -> Self {
        Self {
            pool_token_supply: INITIAL_SWAP_POOL_AMOUNT,
        }
    }

    /// Liquidity tokens for pool tokens, returns None if output is less than 1
    fn liquidity_tokens(&self, pool_tokens: u64, total_liquidity_tokens: u64) -> Option<u64> {
        pool_tokens
            .checked_mul(total_liquidity_tokens)?
            .checked_div(self.pool_token_supply)
            .and_then(map_zero_to_none)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_pool_amount() {
        let token_converter = RelativePoolTokenConverter::new_pool();
        assert_eq!(token_converter.pool_token_supply, INITIAL_SWAP_POOL_AMOUNT);
    }

    fn check_liquidity_pool_token_rate(
        token_a: u64,
        deposit: u64,
        supply: u64,
        expected: Option<u64>,
    ) {
        let calculator = RelativePoolTokenConverter::new_existing(supply);
        assert_eq!(calculator.liquidity_tokens(deposit, token_a), expected);
    }

    #[test]
    fn issued_tokens() {
        check_liquidity_pool_token_rate(2, 5, 10, Some(1));
        check_liquidity_pool_token_rate(10, 5, 10, Some(5));
        check_liquidity_pool_token_rate(5, 5, 10, Some(2));
        check_liquidity_pool_token_rate(5, 5, 10, Some(2));
        check_liquidity_pool_token_rate(u64::MAX, 5, 10, None);
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
        assert_eq!(
            result.new_destination_amount,
            swap_destination_amount - amount_swapped
        );
    }
}
