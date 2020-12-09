//! Swap calculations

use crate::{curve::math::PreciseNumber, error::SwapError};
use std::fmt::Debug;

/// Initial amount of pool tokens for swap contract, hard-coded to something
/// "sensible" given a maximum of u128.
/// Note that on Ethereum, Uniswap uses the geometric mean of all provided
/// input amounts, and Balancer uses 100 * 10 ^ 18.
pub const INITIAL_SWAP_POOL_AMOUNT: u128 = 1_000_000_000;

/// Hardcode the number of token types in a pool, used to calculate the
/// equivalent pool tokens for the owner trading fee.
pub const TOKENS_IN_POOL: u128 = 2;

/// Helper function for mapping to SwapError::CalculationFailure
pub fn map_zero_to_none(x: u128) -> Option<u128> {
    if x == 0 {
        None
    } else {
        Some(x)
    }
}

/// The direction of a trade, since curves can be specialized to treat each
/// token differently (by adding offsets or weights)
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TradeDirection {
    /// Input token A, output token B
    AtoB,
    /// Input token B, output token A
    BtoA,
}

/// Encodes all results of swapping from a source token to a destination token
#[derive(Debug, PartialEq)]
pub struct SwapWithoutFeesResult {
    /// Amount of source token swapped
    pub source_amount_swapped: u128,
    /// Amount of destination token swapped
    pub destination_amount_swapped: u128,
}

/// Encodes results of depositing both sides at once
#[derive(Debug, PartialEq)]
pub struct TradingTokenResult {
    /// Amount of token
    pub token_a_amount: u128,
    /// Amount of destination token swapped
    pub token_b_amount: u128,
}

/// Trait for packing of trait objects, required because structs that implement
/// `Pack` cannot be used as trait objects (as `dyn Pack`).
pub trait DynPack {
    /// Only required function is to pack given a trait object
    fn pack_into_slice(&self, dst: &mut [u8]);
}

/// Trait representing operations required on a swap curve
pub trait CurveCalculator: Debug + DynPack {
    /// Calculate how much destination token will be provided given an amount
    /// of source token.
    fn swap_without_fees(
        &self,
        source_amount: u128,
        swap_source_amount: u128,
        swap_destination_amount: u128,
        trade_direction: TradeDirection,
    ) -> Option<SwapWithoutFeesResult>;

    /// Get the supply for a new pool
    /// The default implementation is a Balancer-style fixed initial supply
    fn new_pool_supply(&self) -> u128 {
        INITIAL_SWAP_POOL_AMOUNT
    }

    /// Get the amount of trading tokens for the given amount of pool tokens,
    /// provided the total trading tokens and supply of pool tokens.
    ///
    /// The default implementation is a simple ratio calculation for how many
    /// trading tokens correspond to a certain number of pool tokens
    fn pool_tokens_to_trading_tokens(
        &self,
        pool_tokens: u128,
        pool_token_supply: u128,
        swap_token_a_amount: u128,
        swap_token_b_amount: u128,
    ) -> Option<TradingTokenResult> {
        let token_a_amount = pool_tokens
            .checked_mul(swap_token_a_amount)?
            .checked_div(pool_token_supply)?;
        let token_b_amount = pool_tokens
            .checked_mul(swap_token_b_amount)?
            .checked_div(pool_token_supply)?;
        Some(TradingTokenResult {
            token_a_amount,
            token_b_amount,
        })
    }

    /// Get the amount of pool tokens for the given amount of token A or B
    /// See the concept for the calculation at:
    /// https://balancer.finance/whitepaper/#single-asset-deposit
    fn trading_tokens_to_pool_tokens(
        &self,
        source_amount: u128,
        swap_token_a_amount: u128,
        swap_token_b_amount: u128,
        pool_supply: u128,
        trade_direction: TradeDirection,
    ) -> Option<u128> {
        let swap_source_amount = match trade_direction {
            TradeDirection::AtoB => swap_token_a_amount,
            TradeDirection::BtoA => swap_token_b_amount,
        };
        let swap_source_amount = PreciseNumber::new(swap_source_amount)?;
        let source_amount = PreciseNumber::new(source_amount)?;
        let ratio = source_amount.checked_div(&swap_source_amount)?;
        let one = PreciseNumber::new(1)?;
        let two = PreciseNumber::new(2)?;
        let base = one.checked_add(&ratio)?;
        let guess = base.checked_div(&two)?;
        let root = base
            .newtonian_root_approximation(&two, guess)?
            .checked_sub(&one)?;
        let pool_supply = PreciseNumber::new(pool_supply)?;
        pool_supply.checked_mul(&root)?.to_imprecise()
    }

    /// Validate that the given curve has no bad parameters
    fn validate(&self) -> Result<(), SwapError>;

    /// Validate the given supply on init, helpful for curves that do or don't
    /// allow zero supply on one side
    fn validate_supply(&self, token_a_amount: u64, token_b_amount: u64) -> Result<(), SwapError> {
        if token_a_amount == 0 {
            return Err(SwapError::EmptySupply);
        }
        if token_b_amount == 0 {
            return Err(SwapError::EmptySupply);
        }
        Ok(())
    }

    /// Some curves will function best and prevent attacks if we prevent
    /// deposits after initialization
    fn allows_deposits(&self) -> bool {
        true
    }
}

#[cfg(test)]
pub mod test {
    use super::*;

    /// Check that two numbers are within 1 of each other
    fn almost_equal(a: u128, b: u128) {
        if a >= b {
            assert!(a - b <= 1);
        } else {
            assert!(b - a <= 1);
        }
    }

    pub fn check_pool_token_conversion(
        curve: &dyn CurveCalculator,
        swap_token_a_amount: u128,
        swap_token_b_amount: u128,
        token_a_amount: u128,
    ) {
        // check that depositing token A is the same as swapping for token B
        // and depositing the result
        let swap_results = curve
            .swap_without_fees(
                token_a_amount,
                swap_token_a_amount,
                swap_token_b_amount,
                TradeDirection::AtoB,
            )
            .unwrap();
        let token_a_amount = swap_results.source_amount_swapped;
        let token_b_amount = swap_results.destination_amount_swapped;
        let pool_supply = curve.new_pool_supply();
        let pool_tokens_from_a = curve
            .trading_tokens_to_pool_tokens(
                token_a_amount,
                swap_token_a_amount + token_a_amount,
                swap_token_b_amount,
                pool_supply,
                TradeDirection::AtoB,
            )
            .unwrap();
        let pool_tokens_from_b = curve
            .trading_tokens_to_pool_tokens(
                token_b_amount,
                swap_token_a_amount + token_a_amount,
                swap_token_b_amount,
                pool_supply,
                TradeDirection::BtoA,
            )
            .unwrap();
        let deposit_token_a = curve
            .pool_tokens_to_trading_tokens(
                pool_tokens_from_a,
                pool_supply + pool_tokens_from_a,
                swap_token_a_amount,
                swap_token_b_amount,
            )
            .unwrap();

        let deposit_token_b = curve
            .pool_tokens_to_trading_tokens(
                pool_tokens_from_b,
                pool_supply + pool_tokens_from_b,
                swap_token_a_amount,
                swap_token_b_amount,
            )
            .unwrap();

        // They should be within 1 token because truncation
        almost_equal(
            deposit_token_b.token_a_amount,
            deposit_token_a.token_a_amount,
        );
        almost_equal(
            deposit_token_b.token_b_amount,
            deposit_token_b.token_b_amount,
        );
    }
}
