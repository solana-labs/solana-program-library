//! The Uniswap invariant calculator.

use solana_program::{
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
};

use crate::curve::calculator::{
    calculate_fee, map_zero_to_none, CurveCalculator, DynPack, SwapResult,
};
use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use std::convert::TryFrom;

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
        let owner_fee = self.owner_trading_fee(source_amount)?;

        let source_amount_less_fee = source_amount
            .checked_sub(trade_fee)?
            .checked_sub(owner_fee)?;
        let invariant = swap_source_amount.checked_mul(swap_destination_amount)?;
        let mut new_source_amount_less_fee =
            swap_source_amount.checked_add(source_amount_less_fee)?;
        let mut new_destination_amount = invariant.checked_div(new_source_amount_less_fee)?;
        // Ceiling the destination amount if there's any remainder, which will
        // almost always be the case.
        let remainder = invariant.checked_rem_euclid(new_source_amount_less_fee)?;
        if remainder > 0 {
            new_destination_amount = new_destination_amount.checked_add(1)?;
            // now calculate the minimum amount of source token needed to get
            // the destination amount to avoid taking too much from users
            new_source_amount_less_fee = invariant.checked_div(new_destination_amount)?;
            let remainder = invariant.checked_rem_euclid(new_destination_amount)?;
            if remainder > 0 {
                new_source_amount_less_fee = new_source_amount_less_fee.checked_add(1)?;
            }
        }
        let source_amount_swapped = new_source_amount_less_fee
            .checked_add(trade_fee)?
            .checked_add(owner_fee)?
            .checked_sub(swap_source_amount)?;
        let amount_swapped =
            map_zero_to_none(swap_destination_amount.checked_sub(new_destination_amount)?)?;

        // actually add the whole amount coming in
        let new_source_amount = swap_source_amount.checked_add(source_amount_swapped)?;

        Some(SwapResult {
            new_source_amount,
            new_destination_amount,
            source_amount_swapped,
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

    /// Calculate the owner trading fee in trading tokens
    fn owner_trading_fee(&self, trading_tokens: u128) -> Option<u128> {
        calculate_fee(
            trading_tokens,
            u128::try_from(self.owner_trade_fee_numerator).ok()?,
            u128::try_from(self.owner_trade_fee_denominator).ok()?,
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
    fn pack_into_slice(&self, output: &mut [u8]) {
        (self as &dyn DynPack).pack_into_slice(output);
    }

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::curve::calculator::INITIAL_SWAP_POOL_AMOUNT;

    #[test]
    fn initial_pool_amount() {
        let trade_fee_numerator = 0;
        let trade_fee_denominator = 1;
        let owner_trade_fee_numerator = 0;
        let owner_trade_fee_denominator = 1;
        let owner_withdraw_fee_numerator = 0;
        let owner_withdraw_fee_denominator = 1;
        let host_fee_numerator = 0;
        let host_fee_denominator = 1;
        let calculator = ConstantProductCurve {
            trade_fee_numerator,
            trade_fee_denominator,
            owner_trade_fee_numerator,
            owner_trade_fee_denominator,
            owner_withdraw_fee_numerator,
            owner_withdraw_fee_denominator,
            host_fee_numerator,
            host_fee_denominator,
        };
        assert_eq!(calculator.new_pool_supply(), INITIAL_SWAP_POOL_AMOUNT);
    }

    fn check_pool_token_rate(token_a: u128, deposit: u128, supply: u128, expected: Option<u128>) {
        let trade_fee_numerator = 0;
        let trade_fee_denominator = 1;
        let owner_trade_fee_numerator = 0;
        let owner_trade_fee_denominator = 1;
        let owner_withdraw_fee_numerator = 0;
        let owner_withdraw_fee_denominator = 1;
        let host_fee_numerator = 0;
        let host_fee_denominator = 1;
        let calculator = ConstantProductCurve {
            trade_fee_numerator,
            trade_fee_denominator,
            owner_trade_fee_numerator,
            owner_trade_fee_denominator,
            owner_withdraw_fee_numerator,
            owner_withdraw_fee_denominator,
            host_fee_numerator,
            host_fee_denominator,
        };
        assert_eq!(
            calculator.pool_tokens_to_trading_tokens(deposit, supply, token_a),
            expected
        );
    }

    #[test]
    fn trading_token_conversion() {
        check_pool_token_rate(2, 5, 10, Some(1));
        check_pool_token_rate(10, 5, 10, Some(5));
        check_pool_token_rate(5, 5, 10, Some(2));
        check_pool_token_rate(5, 5, 10, Some(2));
        check_pool_token_rate(u128::MAX, 5, 10, None);
    }

    #[test]
    fn constant_product_swap_calculation_trade_fee() {
        // calculation on https://github.com/solana-labs/solana-program-library/issues/341
        let swap_source_amount = 1000;
        let swap_destination_amount = 50000;
        let trade_fee_numerator = 1;
        let trade_fee_denominator = 100;
        let owner_trade_fee_numerator = 0;
        let owner_trade_fee_denominator = 0;
        let owner_withdraw_fee_numerator = 0;
        let owner_withdraw_fee_denominator = 0;
        let host_fee_numerator = 0;
        let host_fee_denominator = 0;
        let source_amount = 100;
        let curve = ConstantProductCurve {
            trade_fee_numerator,
            trade_fee_denominator,
            owner_trade_fee_numerator,
            owner_trade_fee_denominator,
            owner_withdraw_fee_numerator,
            owner_withdraw_fee_denominator,
            host_fee_numerator,
            host_fee_denominator,
        };
        let result = curve
            .swap(source_amount, swap_source_amount, swap_destination_amount)
            .unwrap();
        assert_eq!(result.new_source_amount, 1100);
        assert_eq!(result.amount_swapped, 4504);
        assert_eq!(result.new_destination_amount, 45496);
        assert_eq!(result.trade_fee, 1);
        assert_eq!(result.owner_fee, 0);
    }

    #[test]
    fn constant_product_swap_calculation_owner_fee() {
        // calculation on https://github.com/solana-labs/solana-program-library/issues/341
        let swap_source_amount = 1000;
        let swap_destination_amount = 50000;
        let trade_fee_numerator = 0;
        let trade_fee_denominator = 0;
        let owner_trade_fee_numerator = 1;
        let owner_trade_fee_denominator = 100;
        let owner_withdraw_fee_numerator = 0;
        let owner_withdraw_fee_denominator = 0;
        let host_fee_numerator = 0;
        let host_fee_denominator = 0;
        let source_amount: u128 = 100;
        let curve = ConstantProductCurve {
            trade_fee_numerator,
            trade_fee_denominator,
            owner_trade_fee_numerator,
            owner_trade_fee_denominator,
            owner_withdraw_fee_numerator,
            owner_withdraw_fee_denominator,
            host_fee_numerator,
            host_fee_denominator,
        };
        let result = curve
            .swap(source_amount, swap_source_amount, swap_destination_amount)
            .unwrap();
        assert_eq!(result.new_source_amount, 1100);
        assert_eq!(result.amount_swapped, 4504);
        assert_eq!(result.new_destination_amount, 45496);
        assert_eq!(result.trade_fee, 0);
        assert_eq!(result.owner_fee, 1);
    }

    #[test]
    fn constant_product_swap_no_fee() {
        let swap_source_amount: u128 = 1000;
        let swap_destination_amount: u128 = 50000;
        let source_amount: u128 = 100;
        let curve = ConstantProductCurve::default();
        let result = curve
            .swap(source_amount, swap_source_amount, swap_destination_amount)
            .unwrap();
        assert_eq!(result.new_source_amount, 1100);
        assert_eq!(result.amount_swapped, 4545);
        assert_eq!(result.new_destination_amount, 45455);
    }

    #[test]
    fn pack_constant_product_curve() {
        let trade_fee_numerator = 1;
        let trade_fee_denominator = 4;
        let owner_trade_fee_numerator = 2;
        let owner_trade_fee_denominator = 5;
        let owner_withdraw_fee_numerator = 4;
        let owner_withdraw_fee_denominator = 10;
        let host_fee_numerator = 4;
        let host_fee_denominator = 10;
        let curve = ConstantProductCurve {
            trade_fee_numerator,
            trade_fee_denominator,
            owner_trade_fee_numerator,
            owner_trade_fee_denominator,
            owner_withdraw_fee_numerator,
            owner_withdraw_fee_denominator,
            host_fee_numerator,
            host_fee_denominator,
        };

        let mut packed = [0u8; ConstantProductCurve::LEN];
        Pack::pack_into_slice(&curve, &mut packed[..]);
        let unpacked = ConstantProductCurve::unpack(&packed).unwrap();
        assert_eq!(curve, unpacked);

        let mut packed = vec![];
        packed.extend_from_slice(&trade_fee_numerator.to_le_bytes());
        packed.extend_from_slice(&trade_fee_denominator.to_le_bytes());
        packed.extend_from_slice(&owner_trade_fee_numerator.to_le_bytes());
        packed.extend_from_slice(&owner_trade_fee_denominator.to_le_bytes());
        packed.extend_from_slice(&owner_withdraw_fee_numerator.to_le_bytes());
        packed.extend_from_slice(&owner_withdraw_fee_denominator.to_le_bytes());
        packed.extend_from_slice(&host_fee_numerator.to_le_bytes());
        packed.extend_from_slice(&host_fee_denominator.to_le_bytes());
        let unpacked = ConstantProductCurve::unpack(&packed).unwrap();
        assert_eq!(curve, unpacked);
    }

    fn test_truncation(
        curve: &ConstantProductCurve,
        source_amount: u128,
        swap_source_amount: u128,
        swap_destination_amount: u128,
        expected_source_amount_swapped: u128,
        expected_destination_amount_swapped: u128,
    ) {
        let result = curve
            .swap(source_amount, swap_source_amount, swap_destination_amount)
            .unwrap();
        assert_eq!(result.source_amount_swapped, expected_source_amount_swapped);
        assert_eq!(result.amount_swapped, expected_destination_amount_swapped)
    }

    #[test]
    fn constant_product_swap_truncation() {
        let trade_fee_numerator = 25;
        let trade_fee_denominator = 10000;
        let owner_trade_fee_numerator = 5;
        let owner_trade_fee_denominator = 10000;
        let owner_withdraw_fee_numerator = 0;
        let owner_withdraw_fee_denominator = 0;
        let host_fee_numerator = 20;
        let host_fee_denominator = 100;
        let curve = ConstantProductCurve {
            trade_fee_numerator,
            trade_fee_denominator,
            owner_trade_fee_numerator,
            owner_trade_fee_denominator,
            owner_withdraw_fee_numerator,
            owner_withdraw_fee_denominator,
            host_fee_numerator,
            host_fee_denominator,
        };
        // much too small
        assert!(curve
            .swap(12u128, 70_000_000_000u128, 4_000_000u128)
            .is_none()); // spot: 10 * 4m / 70b = 0

        // for these tests, since the amounts are so small, 2 tokens should be
        // subtracted from the actual calculation for fees
        let tests = [
            (
                12u128,
                4_000_000u128,
                70_000_000_000u128,
                12u128,
                174999u128,
            ), // spot: 10 * 70b / 4m = 175000
            (12u128, 30_000u128, 20_000u128, 12u128, 6u128), // spot: 10 * 2 / 3 = 6.6666
            (11u128, 30_000u128, 20_000u128, 10u128, 5u128), // spot: 9 * 2 / 3 = 6, can also get 6 tokens out with 8 in
            (12u128, 20_000u128, 30_000u128, 12u128, 14u128), // spot: 10 * 3 / 2 = 15
            (102u128, 60_000u128, 30_000u128, 101u128, 49u128), // spot: 100 * 3 / 6 = 50, can also get 49 tokens out with 99 in
        ];
        for (
            source_amount,
            swap_source_amount,
            swap_destination_amount,
            expected_source_amount,
            expected_destination_amount,
        ) in tests.iter()
        {
            test_truncation(
                &curve,
                *source_amount,
                *swap_source_amount,
                *swap_destination_amount,
                *expected_source_amount,
                *expected_destination_amount,
            );
        }
    }
}
