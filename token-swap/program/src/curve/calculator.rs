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

impl TradeDirection {
    /// Given a trade direction, gives the opposite direction of the trade, so
    /// A to B becomes B to A, and vice versa
    pub fn opposite(&self) -> TradeDirection {
        match self {
            TradeDirection::AtoB => TradeDirection::BtoA,
            TradeDirection::BtoA => TradeDirection::AtoB,
        }
    }
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

    /// Get the amount of pool tokens for the given amount of token A or B.
    ///
    /// This is used for single-sided deposits or withdrawals and owner trade
    /// fee calculation. It essentially performs a swap followed by a deposit,
    /// or a withdrawal followed by a swap.  Because a swap is implicitly
    /// performed, this will change the spot price of the pool.
    ///
    /// See more background for the calculation at:
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
        let base = one.checked_add(&ratio)?;
        let root = base.sqrt()?.checked_sub(&one)?;
        let pool_supply = PreciseNumber::new(pool_supply)?;
        pool_supply.checked_mul(&root)?.to_imprecise()
    }

    /// Validate that the given curve has no invalid parameters
    fn validate(&self) -> Result<(), SwapError>;

    /// Validate the given supply on initialization. This is useful for curves
    /// that allow zero supply on one or both sides, since the standard constant
    /// product curve must have a non-zero supply on both sides.
    fn validate_supply(&self, token_a_amount: u64, token_b_amount: u64) -> Result<(), SwapError> {
        if token_a_amount == 0 {
            return Err(SwapError::EmptySupply);
        }
        if token_b_amount == 0 {
            return Err(SwapError::EmptySupply);
        }
        Ok(())
    }

    /// Some curves function best and prevent attacks if we prevent deposits
    /// after initialization.  For example, the offset curve in `offset.rs`,
    /// which fakes supply on one side of the swap, allows the swap creator
    /// to steal value from all other depositors.
    fn allows_deposits(&self) -> bool {
        true
    }

    /// Calculates the total normalized value of the curve given the liquidity
    /// parameters.
    ///
    /// This value must have the dimension of `tokens ^ 1` For example, the
    /// standard Uniswap invariant has dimension `tokens ^ 2` since we are
    /// multiplying two token values together.  In order to normalize it, we
    /// also need to take the square root.
    ///
    /// This is useful for testing the curves, to make sure that value is not
    /// lost on any trade.  It can also be used to find out the relative value
    /// of pool tokens or liquidity tokens.
    ///
    /// The default implementation for this function gives the square root of
    /// the Uniswap invariant.
    fn normalized_value(
        &self,
        swap_token_a_amount: u128,
        swap_token_b_amount: u128,
    ) -> Option<u128> {
        let swap_token_a_amount = PreciseNumber::new(swap_token_a_amount)?;
        let swap_token_b_amount = PreciseNumber::new(swap_token_b_amount)?;
        swap_token_a_amount
            .checked_mul(&swap_token_b_amount)?
            .sqrt()?
            .to_imprecise()
    }
}

/// Test helpers for curves
#[cfg(any(test, fuzzing))]
pub mod test {
    use super::*;

    /// The epsilon for most curves when performing the conversion test,
    /// comparing a one-sided deposit to a swap + deposit.
    pub const CONVERSION_BASIS_POINTS_GUARANTEE: u128 = 50;

    /// Test function to check that depositing token A is the same as swapping
    /// half for token B and depositing both.
    /// Since calculations use unsigned integers, there will be truncation at
    /// some point, meaning we can't have perfect equality.
    /// We guarantee that the relative error between depositing one side and
    /// performing a swap plus deposit will be at most some epsilon provided by
    /// the curve. Most curves guarantee accuracy within 0.5%.
    pub fn check_pool_token_conversion(
        curve: &dyn CurveCalculator,
        source_token_amount: u128,
        swap_source_amount: u128,
        swap_destination_amount: u128,
        trade_direction: TradeDirection,
        pool_supply: u128,
        epsilon_in_basis_points: u128,
    ) {
        let amount_to_swap = source_token_amount / 2;
        let results = curve
            .swap_without_fees(
                amount_to_swap,
                swap_source_amount,
                swap_destination_amount,
                trade_direction,
            )
            .unwrap();
        let opposite_direction = trade_direction.opposite();
        let (swap_token_a_amount, swap_token_b_amount) = match trade_direction {
            TradeDirection::AtoB => (swap_source_amount, swap_destination_amount),
            TradeDirection::BtoA => (swap_destination_amount, swap_source_amount),
        };

        // base amount
        let pool_tokens_from_one_side = curve
            .trading_tokens_to_pool_tokens(
                source_token_amount,
                swap_token_a_amount,
                swap_token_b_amount,
                pool_supply,
                trade_direction,
            )
            .unwrap();

        // perform both separately, updating amounts accordingly
        let (swap_token_a_amount, swap_token_b_amount) = match trade_direction {
            TradeDirection::AtoB => (
                swap_source_amount + results.source_amount_swapped,
                swap_destination_amount - results.destination_amount_swapped,
            ),
            TradeDirection::BtoA => (
                swap_destination_amount - results.destination_amount_swapped,
                swap_source_amount + results.source_amount_swapped,
            ),
        };
        let pool_tokens_from_source = curve
            .trading_tokens_to_pool_tokens(
                source_token_amount - results.source_amount_swapped,
                swap_token_a_amount,
                swap_token_b_amount,
                pool_supply,
                trade_direction,
            )
            .unwrap();
        let pool_tokens_from_destination = curve
            .trading_tokens_to_pool_tokens(
                results.destination_amount_swapped,
                swap_token_a_amount,
                swap_token_b_amount,
                pool_supply + pool_tokens_from_source,
                opposite_direction,
            )
            .unwrap();

        let pool_tokens_total_separate = pool_tokens_from_source + pool_tokens_from_destination;

        // slippage due to rounding or truncation errors
        let epsilon = std::cmp::max(
            1,
            pool_tokens_total_separate * epsilon_in_basis_points / 10000,
        );
        let difference = if pool_tokens_from_one_side >= pool_tokens_total_separate {
            pool_tokens_from_one_side - pool_tokens_total_separate
        } else {
            pool_tokens_total_separate - pool_tokens_from_one_side
        };
        assert!(difference <= epsilon);
    }

    /// Test function checking that a swap never reduces the overall value of
    /// the pool.
    ///
    /// Since curve calculations use unsigned integers, there is potential for
    /// truncation at some point, meaning a potential for value to be lost in
    /// either direction if too much is given to the swapper.
    ///
    /// This test guarantees that the relative change in value will be at most
    /// 1 normalized token, and that the value will never decrease from a trade.
    pub fn check_curve_value_from_swap(
        curve: &dyn CurveCalculator,
        source_token_amount: u128,
        swap_source_amount: u128,
        swap_destination_amount: u128,
        trade_direction: TradeDirection,
    ) {
        let results = curve
            .swap_without_fees(
                source_token_amount,
                swap_source_amount,
                swap_destination_amount,
                trade_direction,
            )
            .unwrap();

        let (swap_token_a_amount, swap_token_b_amount) = match trade_direction {
            TradeDirection::AtoB => (swap_source_amount, swap_destination_amount),
            TradeDirection::BtoA => (swap_destination_amount, swap_source_amount),
        };
        let previous_value = curve
            .normalized_value(swap_token_a_amount, swap_token_b_amount)
            .unwrap();

        let new_swap_source_amount = swap_source_amount
            .checked_add(results.source_amount_swapped)
            .unwrap();
        let new_swap_destination_amount = swap_destination_amount
            .checked_sub(results.destination_amount_swapped)
            .unwrap();
        let (swap_token_a_amount, swap_token_b_amount) = match trade_direction {
            TradeDirection::AtoB => (new_swap_source_amount, new_swap_destination_amount),
            TradeDirection::BtoA => (new_swap_destination_amount, new_swap_source_amount),
        };

        let new_value = curve
            .normalized_value(swap_token_a_amount, swap_token_b_amount)
            .unwrap();
        assert!(new_value >= previous_value);

        let epsilon = 1; // Extremely close!
        let difference = new_value - previous_value;
        assert!(difference <= epsilon);
    }
}
