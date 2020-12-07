//! Simple constant price swap curve, set at init

use crate::{
    curve::calculator::{
        CurveCalculator, DynPack, SwapWithoutFeesResult, TradeDirection, TradingTokenResult,
    },
    error::SwapError,
};
use solana_program::{
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
};

/// ConstantPriceCurve struct implementing CurveCalculator
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ConstantPriceCurve;

impl CurveCalculator for ConstantPriceCurve {
    /// Constant price curve always returns 1:1
    fn swap_without_fees(
        &self,
        source_amount: u128,
        _swap_source_amount: u128,
        _swap_destination_amount: u128,
        _trade_direction: TradeDirection,
    ) -> Option<SwapWithoutFeesResult> {
        Some(SwapWithoutFeesResult {
            source_amount_swapped: source_amount,
            destination_amount_swapped: source_amount,
        })
    }

    /// Get the amount of trading tokens for the given amount of pool tokens,
    /// provided the total trading tokens and supply of pool tokens.
    /// For the constant price curve, the total value of the pool is weighted
    /// by the price of token B.
    fn pool_tokens_to_trading_tokens(
        &self,
        pool_tokens: u128,
        pool_token_supply: u128,
        swap_token_a_amount: u128,
        swap_token_b_amount: u128,
    ) -> Option<TradingTokenResult> {
        // Split the pool tokens in half, send half as token A, half as token B
        let token_a_pool_tokens = pool_tokens.checked_div(2)?;
        let token_b_pool_tokens = pool_tokens.checked_sub(token_a_pool_tokens)?;

        let token_b_price = self.token_b_price as u128;
        let total_value = swap_token_b_amount.checked_mul(token_b_price)?.checked_add(swap_token_a_amount)?;

        let token_a_amount = token_a_pool_tokens
            .checked_mul(total_value)?
            .checked_div(pool_token_supply)?;
        let token_b_amount = token_b_pool_tokens
            .checked_mul(total_value)?
            .checked_div(token_b_price)?
            .checked_div(pool_token_supply)?;
        Some(TradingTokenResult {
            token_a_amount,
            token_b_amount,
        })
    }

    /// Get the amount of pool tokens for the given amount of token A and B
    /// For the constant price curve, the total value of the pool is weighted
    /// by the price of token B.
    fn trading_tokens_to_pool_tokens(
        &self,
        token_a_amount: u128,
        swap_token_a_amount: u128,
        token_b_amount: u128,
        swap_token_b_amount: u128,
        pool_supply: u128,
    ) -> Option<u128> {
        let token_b_price = self.token_b_price as u128;
        let given_value = token_b_amount.checked_mul(token_b_price)?.checked_add(token_a_amount)?;
        let total_value = swap_token_b_amount.checked_mul(token_b_price)?.checked_add(swap_token_a_amount)?;
        pool_supply.checked_mul(given_value)?.checked_div(total_value)
    }

    fn validate(&self) -> Result<(), SwapError> {
        Ok(())
    }

    fn validate_supply(&self, token_a_amount: u64, _token_b_amount: u64) -> Result<(), SwapError> {
        if token_a_amount == 0 {
            return Err(SwapError::EmptySupply);
        }
        Ok(())
    }
}

/// IsInitialized is required to use `Pack::pack` and `Pack::unpack`
impl IsInitialized for ConstantPriceCurve {
    fn is_initialized(&self) -> bool {
        true
    }
}
impl Sealed for ConstantPriceCurve {}
impl Pack for ConstantPriceCurve {
    const LEN: usize = 0;
    fn pack_into_slice(&self, output: &mut [u8]) {
        (self as &dyn DynPack).pack_into_slice(output);
    }

    fn unpack_from_slice(_input: &[u8]) -> Result<ConstantPriceCurve, ProgramError> {
        Ok(Self {})
    }
}

impl DynPack for ConstantPriceCurve {
    fn pack_into_slice(&self, _output: &mut [u8]) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn swap_calculation_no_price() {
        let swap_source_amount: u128 = 0;
        let swap_destination_amount: u128 = 0;
        let source_amount: u128 = 100;
        let curve = ConstantPriceCurve {};

        let expected_result = SwapWithoutFeesResult {
            source_amount_swapped: source_amount,
            destination_amount_swapped: source_amount,
        };

        let result = curve
            .swap_without_fees(
                source_amount,
                swap_source_amount,
                swap_destination_amount,
                TradeDirection::AtoB,
            )
            .unwrap();
        assert_eq!(result, expected_result);

        let result = curve
            .swap_without_fees(
                source_amount,
                swap_source_amount,
                swap_destination_amount,
                TradeDirection::BtoA,
            )
            .unwrap();
        assert_eq!(result, expected_result);
    }

    #[test]
    fn pack_flat_curve() {
        let curve = ConstantPriceCurve {};

        let mut packed = [0u8; ConstantPriceCurve::LEN];
        Pack::pack_into_slice(&curve, &mut packed[..]);
        let unpacked = ConstantPriceCurve::unpack(&packed).unwrap();
        assert_eq!(curve, unpacked);

        let packed = vec![];
        let unpacked = ConstantPriceCurve::unpack(&packed).unwrap();
        assert_eq!(curve, unpacked);
    }

    #[test]
    fn pool_token_conversion() {
        let token_b_price = 10_000;
        let swap_token_a_amount = 1_000_000;
        let swap_token_b_amount = 1;
        let curve = ConstantPriceCurve { token_b_price: token_b_price as u64 };
        let token_b_amount = 10;
        let token_a_amount = token_b_amount * token_b_price;
        let pool_supply = curve.new_pool_supply();
        let pool_tokens = curve.trading_tokens_to_pool_tokens(
            token_a_amount,
            swap_token_a_amount,
            token_b_amount,
            swap_token_b_amount,
            pool_supply,
        ).unwrap();
        let results = curve.pool_tokens_to_trading_tokens(
            pool_tokens,
            pool_supply,
            swap_token_a_amount,
            swap_token_b_amount,
        ).unwrap();
        assert_eq!(results.token_a_amount, token_a_amount - 1); // as long as we don't create more, we're good
        assert_eq!(results.token_b_amount, token_b_amount);
    }
}
