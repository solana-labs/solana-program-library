//! The Uniswap invariant calculator.

use solana_program::{
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
};

use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use std::convert::{TryFrom};
use super::shared::{CurveCalculator, DynPack, SwapResult, calculate_fee, map_zero_to_none};

/// ConstantProductCurve struct implementing CurveCalculator
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ConstantProductCurve {
    /// Trade fee numerator
    pub trade_fee_numerator: u64,
    /// Trade fee denominator
    pub trade_fee_denominator: u64,
    /// Owner trade fee numerator
    pub owner_trade_fee_numerator: u64,
    /// Owner trade fee denominator
    pub owner_trade_fee_denominator: u64,
    /// Owner withdraw fee numerator
    pub owner_withdraw_fee_numerator: u64,
    /// Owner withdraw fee denominator
    pub owner_withdraw_fee_denominator: u64,
    /// Host trading fee numerator
    pub host_fee_numerator: u64,
    /// Host trading fee denominator
    pub host_fee_denominator: u64,
}

impl CurveCalculator for ConstantProductCurve {
    /// Constant product swap ensures x * y = constant
    fn swap(
        &self,
        source_amount: u128,
        swap_source_amount: u128,
        swap_destination_amount: u128,
    ) -> Option<SwapResult> {
        // debit the fee to calculate the amount swapped
        let trade_fee = self.trading_fee(source_amount)?;
        let owner_fee = calculate_fee(
            source_amount,
            u128::try_from(self.owner_trade_fee_numerator).ok()?,
            u128::try_from(self.owner_trade_fee_denominator).ok()?,
        )?;

        let invariant = swap_source_amount.checked_mul(swap_destination_amount)?;
        let new_source_amount_less_fee = swap_source_amount
            .checked_add(source_amount)?
            .checked_sub(trade_fee)?
            .checked_sub(owner_fee)?;
        let new_destination_amount = invariant.checked_div(new_source_amount_less_fee)?;
        let amount_swapped =
            map_zero_to_none(swap_destination_amount.checked_sub(new_destination_amount)?)?;

        // actually add the whole amount coming in
        let new_source_amount = swap_source_amount.checked_add(source_amount)?;
        Some(SwapResult {
            new_source_amount,
            new_destination_amount,
            amount_swapped,
            trade_fee,
            owner_fee,
        })
    }

    /// Calculate the withdraw fee in pool tokens
    fn owner_withdraw_fee(&self, pool_tokens: u128) -> Option<u128> {
        calculate_fee(
            pool_tokens,
            u128::try_from(self.owner_withdraw_fee_numerator).ok()?,
            u128::try_from(self.owner_withdraw_fee_denominator).ok()?,
        )
    }

    /// Calculate the trading fee in trading tokens
    fn trading_fee(&self, trading_tokens: u128) -> Option<u128> {
        calculate_fee(
            trading_tokens,
            u128::try_from(self.trade_fee_numerator).ok()?,
            u128::try_from(self.trade_fee_denominator).ok()?,
        )
    }

    /// Calculate the host fee based on the owner fee, only used in production
    /// situations where a program is hosted by multiple frontends
    fn host_fee(&self, owner_fee: u128) -> Option<u128> {
        calculate_fee(
            owner_fee,
            u128::try_from(self.host_fee_numerator).ok()?,
            u128::try_from(self.host_fee_denominator).ok()?,
        )
    }
}

/// IsInitialized is required to use `Pack::pack` and `Pack::unpack`
impl IsInitialized for ConstantProductCurve {
    fn is_initialized(&self) -> bool {
        true
    }
}
impl Sealed for ConstantProductCurve {}
impl Pack for ConstantProductCurve {
    const LEN: usize = 64;
    fn unpack_from_slice(input: &[u8]) -> Result<ConstantProductCurve, ProgramError> {
        let input = array_ref![input, 0, 64];
        #[allow(clippy::ptr_offset_with_cast)]
        let (
            trade_fee_numerator,
            trade_fee_denominator,
            owner_trade_fee_numerator,
            owner_trade_fee_denominator,
            owner_withdraw_fee_numerator,
            owner_withdraw_fee_denominator,
            host_fee_numerator,
            host_fee_denominator,
        ) = array_refs![input, 8, 8, 8, 8, 8, 8, 8, 8];
        Ok(Self {
            trade_fee_numerator: u64::from_le_bytes(*trade_fee_numerator),
            trade_fee_denominator: u64::from_le_bytes(*trade_fee_denominator),
            owner_trade_fee_numerator: u64::from_le_bytes(*owner_trade_fee_numerator),
            owner_trade_fee_denominator: u64::from_le_bytes(*owner_trade_fee_denominator),
            owner_withdraw_fee_numerator: u64::from_le_bytes(*owner_withdraw_fee_numerator),
            owner_withdraw_fee_denominator: u64::from_le_bytes(*owner_withdraw_fee_denominator),
            host_fee_numerator: u64::from_le_bytes(*host_fee_numerator),
            host_fee_denominator: u64::from_le_bytes(*host_fee_denominator),
        })
    }

    fn pack_into_slice(&self, output: &mut [u8]) {
        (self as &dyn DynPack).pack_into_slice(output);
    }
}

impl DynPack for ConstantProductCurve {
    fn pack_into_slice(&self, output: &mut [u8]) {
        let output = array_mut_ref![output, 0, 64];
        let (
            trade_fee_numerator,
            trade_fee_denominator,
            owner_trade_fee_numerator,
            owner_trade_fee_denominator,
            owner_withdraw_fee_numerator,
            owner_withdraw_fee_denominator,
            host_fee_numerator,
            host_fee_denominator,
        ) = mut_array_refs![output, 8, 8, 8, 8, 8, 8, 8, 8];
        *trade_fee_numerator = self.trade_fee_numerator.to_le_bytes();
        *trade_fee_denominator = self.trade_fee_denominator.to_le_bytes();
        *owner_trade_fee_numerator = self.owner_trade_fee_numerator.to_le_bytes();
        *owner_trade_fee_denominator = self.owner_trade_fee_denominator.to_le_bytes();
        *owner_withdraw_fee_numerator = self.owner_withdraw_fee_numerator.to_le_bytes();
        *owner_withdraw_fee_denominator = self.owner_withdraw_fee_denominator.to_le_bytes();
        *host_fee_numerator = self.host_fee_numerator.to_le_bytes();
        *host_fee_denominator = self.host_fee_denominator.to_le_bytes();
    }
}