//! The Uniswap invariant calculator.

use solana_program::{
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
};

use crate::{
    curve::calculator::{
        map_zero_to_none, CurveCalculator, DynPack, SwapWithoutFeesResult, TradeDirection,
    },
    error::SwapError,
};

/// ConstantProductCurve struct implementing CurveCalculator
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ConstantProductCurve;

/// The constant product swap calculation, factored out of its class for reuse
pub fn swap(
    source_amount: u128,
    swap_source_amount: u128,
    swap_destination_amount: u128,
) -> Option<SwapWithoutFeesResult> {
    let invariant = swap_source_amount.checked_mul(swap_destination_amount)?;

    let mut new_swap_source_amount = swap_source_amount.checked_add(source_amount)?;
    let mut new_swap_destination_amount = invariant.checked_div(new_swap_source_amount)?;

    // Ceiling the destination amount if there's any remainder, which will
    // almost always be the case.
    let remainder = invariant.checked_rem(new_swap_source_amount)?;
    if remainder > 0 {
        new_swap_destination_amount = new_swap_destination_amount.checked_add(1)?;
        // now calculate the minimum amount of source token needed to get
        // the destination amount to avoid taking too much from users
        new_swap_source_amount = invariant.checked_div(new_swap_destination_amount)?;
        let remainder = invariant.checked_rem(new_swap_destination_amount)?;
        if remainder > 0 {
            new_swap_source_amount = new_swap_source_amount.checked_add(1)?;
        }
    }

    let source_amount_swapped = new_swap_source_amount.checked_sub(swap_source_amount)?;
    let destination_amount_swapped =
        map_zero_to_none(swap_destination_amount.checked_sub(new_swap_destination_amount)?)?;

    Some(SwapWithoutFeesResult {
        source_amount_swapped,
        destination_amount_swapped,
    })
}

impl CurveCalculator for ConstantProductCurve {
    /// Constant product swap ensures x * y = constant
    fn swap_without_fees(
        &self,
        source_amount: u128,
        swap_source_amount: u128,
        swap_destination_amount: u128,
        _trade_direction: TradeDirection,
    ) -> Option<SwapWithoutFeesResult> {
        swap(source_amount, swap_source_amount, swap_destination_amount)
    }

    fn validate(&self) -> Result<(), SwapError> {
        Ok(())
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
    const LEN: usize = 0;
    fn pack_into_slice(&self, output: &mut [u8]) {
        (self as &dyn DynPack).pack_into_slice(output);
    }

    fn unpack_from_slice(_input: &[u8]) -> Result<ConstantProductCurve, ProgramError> {
        Ok(Self {})
    }
}

impl DynPack for ConstantProductCurve {
    fn pack_into_slice(&self, _output: &mut [u8]) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::curve::calculator::{test::check_pool_token_conversion, INITIAL_SWAP_POOL_AMOUNT};

    #[test]
    fn initial_pool_amount() {
        let calculator = ConstantProductCurve {};
        assert_eq!(calculator.new_pool_supply(), INITIAL_SWAP_POOL_AMOUNT);
    }

    fn check_pool_token_rate(
        token_a: u128,
        token_b: u128,
        deposit: u128,
        supply: u128,
        expected_a: u128,
        expected_b: u128,
    ) {
        let calculator = ConstantProductCurve {};
        let results = calculator
            .pool_tokens_to_trading_tokens(deposit, supply, token_a, token_b)
            .unwrap();
        assert_eq!(results.token_a_amount, expected_a);
        assert_eq!(results.token_b_amount, expected_b);
    }

    #[test]
    fn trading_token_conversion() {
        check_pool_token_rate(2, 49, 5, 10, 1, 24);
        check_pool_token_rate(100, 202, 5, 101, 4, 10);
        check_pool_token_rate(5, 501, 2, 10, 1, 100);
    }

    #[test]
    fn fail_trading_token_conversion() {
        let calculator = ConstantProductCurve {};
        let results = calculator.pool_tokens_to_trading_tokens(5, 10, u128::MAX, 0);
        assert!(results.is_none());
        let results = calculator.pool_tokens_to_trading_tokens(5, 10, 0, u128::MAX);
        assert!(results.is_none());
    }

    #[test]
    fn pack_constant_product_curve() {
        let curve = ConstantProductCurve {};

        let mut packed = [0u8; ConstantProductCurve::LEN];
        Pack::pack_into_slice(&curve, &mut packed[..]);
        let unpacked = ConstantProductCurve::unpack(&packed).unwrap();
        assert_eq!(curve, unpacked);

        let packed = vec![];
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
        let invariant = swap_source_amount * swap_destination_amount;
        let result = curve
            .swap_without_fees(
                source_amount,
                swap_source_amount,
                swap_destination_amount,
                TradeDirection::AtoB,
            )
            .unwrap();
        assert_eq!(result.source_amount_swapped, expected_source_amount_swapped);
        assert_eq!(
            result.destination_amount_swapped,
            expected_destination_amount_swapped
        );
        let new_invariant = (swap_source_amount + result.source_amount_swapped)
            * (swap_destination_amount - result.destination_amount_swapped);
        assert!(new_invariant >= invariant);
    }

    #[test]
    fn constant_product_swap_rounding() {
        let curve = ConstantProductCurve::default();

        // much too small
        assert!(curve
            .swap_without_fees(10, 70_000_000_000, 4_000_000, TradeDirection::AtoB)
            .is_none()); // spot: 10 * 4m / 70b = 0

        let tests: &[(u128, u128, u128, u128, u128)] = &[
            (10, 4_000_000, 70_000_000_000, 10, 174_999), // spot: 10 * 70b / ~4m = 174,999.99
            (20, 30_000 - 20, 10_000, 18, 6), // spot: 20 * 1 / 3.000 = 6.6667 (source can be 18 to get 6 dest.)
            (19, 30_000 - 20, 10_000, 18, 6), // spot: 19 * 1 / 2.999 = 6.3334 (source can be 18 to get 6 dest.)
            (18, 30_000 - 20, 10_000, 18, 6), // spot: 18 * 1 / 2.999 = 6.0001
            (10, 20_000, 30_000, 10, 14),     // spot: 10 * 3 / 2.0010 = 14.99
            (10, 20_000 - 9, 30_000, 10, 14), // spot: 10 * 3 / 2.0001 = 14.999
            (10, 20_000 - 10, 30_000, 10, 15), // spot: 10 * 3 / 2.0000 = 15
            (100, 60_000, 30_000, 99, 49), // spot: 100 * 3 / 6.001 = 49.99 (source can be 99 to get 49 dest.)
            (99, 60_000, 30_000, 99, 49),  // spot: 99 * 3 / 6.001 = 49.49
            (98, 60_000, 30_000, 97, 48), // spot: 98 * 3 / 6.001 = 48.99 (source can be 97 to get 48 dest.)
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

    #[test]
    fn pool_token_conversion() {
        let tests: &[(u128, u128, u128)] = &[
            (1_000_000, 2400112, 100_000),
            (1_000, 100, 100),
            (30, 1_288, 100_000),
            (1_000, 1_288, 100_000),
            (212, 10_000, 100_000),
        ];
        for (swap_token_a_amount, swap_token_b_amount, token_a_amount) in tests.iter() {
            let curve = ConstantProductCurve {};
            check_pool_token_conversion(
                &curve,
                *swap_token_a_amount,
                *swap_token_b_amount,
                *token_a_amount,
            );
        }
    }
}
