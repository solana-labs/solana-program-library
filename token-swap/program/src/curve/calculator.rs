//! Swap calculations

use crate::error::SwapError;
use std::fmt::Debug;

/// Initial amount of pool tokens for swap contract, hard-coded to something
/// "sensible" given a maximum of u128.
/// Note that on Ethereum, Uniswap uses the geometric mean of all provided
/// input amounts, and Balancer uses 100 * 10 ^ 18.
pub const INITIAL_SWAP_POOL_AMOUNT: u128 = 1_000_000_000;

/// Hardcode the number of token types in a pool, used to calculate the
/// equivalent pool tokens for the owner trading fee.
const TOKENS_IN_POOL: u128 = 2;

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
        pool_supply
            .checked_mul(source_amount)?
            .checked_div(swap_source_amount)?
            .checked_div(TOKENS_IN_POOL)
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
}
