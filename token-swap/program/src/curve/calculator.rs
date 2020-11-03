//! Swap calculations

use std::fmt::Debug;

/// Initial amount of pool tokens for swap contract, hard-coded to something
/// "sensible" given a maximum of u128.
/// Note that on Ethereum, Uniswap uses the geometric mean of all provided
/// input amounts, and Balancer uses 100 * 10 ^ 18.
pub const INITIAL_SWAP_POOL_AMOUNT: u128 = 1_000_000_000;

/// Helper function for calcuating swap fee
pub fn calculate_fee(
    token_amount: u128,
    fee_numerator: u128,
    fee_denominator: u128,
) -> Option<u128> {
    if fee_numerator == 0 {
        Some(0)
    } else {
        let fee = token_amount
            .checked_mul(fee_numerator)?
            .checked_div(fee_denominator)?;
        if fee == 0 {
            Some(1) // minimum fee of one token
        } else {
            Some(fee)
        }
    }
}

/// Helper function for mapping to SwapError::CalculationFailure
pub fn map_zero_to_none(x: u128) -> Option<u128> {
    if x == 0 {
        None
    } else {
        Some(x)
    }
}

/// Encodes all results of swapping from a source token to a destination token
pub struct SwapResult {
    /// New amount of source token
    pub new_source_amount: u128,
    /// New amount of destination token
    pub new_destination_amount: u128,
    /// Amount of destination token swapped
    pub amount_swapped: u128,
    /// Amount of source tokens going to pool holders
    pub trade_fee: u128,
    /// Amount of source tokens going to owner
    pub owner_fee: u128,
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
    fn swap(
        &self,
        source_amount: u128,
        swap_source_amount: u128,
        swap_destination_amount: u128,
    ) -> Option<SwapResult>;

    /// Calculate the withdraw fee in pool tokens
    /// Default implementation assumes no fee
    fn owner_withdraw_fee(&self, _pool_tokens: u128) -> Option<u128> {
        Some(0)
    }

    /// Calculate the trading fee in trading tokens
    /// Default implementation assumes no fee
    fn trading_fee(&self, _trading_tokens: u128) -> Option<u128> {
        Some(0)
    }

    /// Calculate the pool token equivalent of the owner fee on trade
    /// See the math at: https://balancer.finance/whitepaper/#single-asset-deposit
    /// For the moment, we do an approximation for the square root.  For numbers
    /// just above 1, simply dividing by 2 brings you very close to the correct
    /// value.
    fn owner_fee_to_pool_tokens(
        &self,
        owner_fee: u128,
        trading_token_amount: u128,
        pool_supply: u128,
        tokens_in_pool: u128,
    ) -> Option<u128> {
        // Get the trading fee incurred if the owner fee is swapped for the other side
        let trade_fee = self.trading_fee(owner_fee)?;
        let owner_fee = owner_fee.checked_sub(trade_fee)?;
        pool_supply
            .checked_mul(owner_fee)?
            .checked_div(trading_token_amount)?
            .checked_div(tokens_in_pool)
    }

    /// Get the supply for a new pool
    /// The default implementation is a Balancer-style fixed initial supply
    fn new_pool_supply(&self) -> u128 {
        INITIAL_SWAP_POOL_AMOUNT
    }

    /// Get the amount of trading tokens for the given amount of pool tokens,
    /// provided the total trading tokens and supply of pool tokens.
    /// The default implementation is a simple ratio calculation for how many
    /// trading tokens correspond to a certain number of pool tokens
    fn pool_tokens_to_trading_tokens(
        &self,
        pool_tokens: u128,
        pool_token_supply: u128,
        total_trading_tokens: u128,
    ) -> Option<u128> {
        pool_tokens
            .checked_mul(total_trading_tokens)?
            .checked_div(pool_token_supply)
            .and_then(map_zero_to_none)
    }

    /// Calculate the host fee based on the owner fee, only used in production
    /// situations where a program is hosted by multiple frontends
    fn host_fee(&self, _owner_fee: u128) -> Option<u128> {
        Some(0)
    }
}
