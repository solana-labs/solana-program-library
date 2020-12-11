//! Simple constant price swap curve, set at init

use crate::{
    curve::calculator::{
        map_zero_to_none, CurveCalculator, DynPack, SwapWithoutFeesResult, TradeDirection,
        TradingTokenResult,
    },
    error::SwapError,
};
use arrayref::{array_mut_ref, array_ref};
use solana_program::{
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
};

/// ConstantPriceCurve struct implementing CurveCalculator
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ConstantPriceCurve {
    /// Amount of token A required to get 1 token B
    pub token_b_price: u64,
}

impl CurveCalculator for ConstantPriceCurve {
    /// Constant price curve always returns 1:1
    fn swap_without_fees(
        &self,
        source_amount: u128,
        _swap_source_amount: u128,
        _swap_destination_amount: u128,
        trade_direction: TradeDirection,
    ) -> Option<SwapWithoutFeesResult> {
        let token_b_price = self.token_b_price as u128;

        let (source_amount_swapped, destination_amount_swapped) = match trade_direction {
            TradeDirection::BtoA => (source_amount, source_amount.checked_mul(token_b_price)?),
            TradeDirection::AtoB => {
                let destination_amount_swapped = source_amount.checked_div(token_b_price)?;
                let mut source_amount_swapped = source_amount;

                // if there is a remainder from buying token B, floor
                // token_a_amount provided to avoid taking too many tokens, but
                // don't recalculate the fees
                let remainder = source_amount_swapped.checked_rem(token_b_price)?;
                if remainder > 0 {
                    source_amount_swapped = source_amount.checked_sub(remainder)?;
                }

                (source_amount_swapped, destination_amount_swapped)
            }
        };
        let source_amount_swapped = map_zero_to_none(source_amount_swapped)?;
        let destination_amount_swapped = map_zero_to_none(destination_amount_swapped)?;
        Some(SwapWithoutFeesResult {
            source_amount_swapped,
            destination_amount_swapped,
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
        let total_value = swap_token_b_amount
            .checked_mul(token_b_price)?
            .checked_add(swap_token_a_amount)?;

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
        source_amount: u128,
        swap_token_a_amount: u128,
        swap_token_b_amount: u128,
        pool_supply: u128,
        trade_direction: TradeDirection,
    ) -> Option<u128> {
        let token_b_price = self.token_b_price as u128;
        let given_value = match trade_direction {
            TradeDirection::AtoB => source_amount,
            TradeDirection::BtoA => source_amount.checked_mul(token_b_price)?,
        };
        let total_value = swap_token_b_amount
            .checked_mul(token_b_price)?
            .checked_add(swap_token_a_amount)?;
        pool_supply
            .checked_mul(given_value)?
            .checked_div(total_value)
    }

    fn validate(&self) -> Result<(), SwapError> {
        if self.token_b_price == 0 {
            Err(SwapError::InvalidCurve)
        } else {
            Ok(())
        }
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
    const LEN: usize = 8;
    fn pack_into_slice(&self, output: &mut [u8]) {
        (self as &dyn DynPack).pack_into_slice(output);
    }

    fn unpack_from_slice(input: &[u8]) -> Result<ConstantPriceCurve, ProgramError> {
        let token_b_price = array_ref![input, 0, 8];
        Ok(Self {
            token_b_price: u64::from_le_bytes(*token_b_price),
        })
    }
}

impl DynPack for ConstantPriceCurve {
    fn pack_into_slice(&self, output: &mut [u8]) {
        let token_b_price = array_mut_ref![output, 0, 8];
        *token_b_price = self.token_b_price.to_le_bytes();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn swap_calculation_no_price() {
        let swap_source_amount: u128 = 0;
        let swap_destination_amount: u128 = 0;
        let source_amount: u128 = 100;
        let token_b_price = 1;
        let curve = ConstantPriceCurve { token_b_price };

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
        let token_b_price = 1_251_258;
        let curve = ConstantPriceCurve { token_b_price };

        let mut packed = [0u8; ConstantPriceCurve::LEN];
        Pack::pack_into_slice(&curve, &mut packed[..]);
        let unpacked = ConstantPriceCurve::unpack(&packed).unwrap();
        assert_eq!(curve, unpacked);

        let mut packed = vec![];
        packed.extend_from_slice(&token_b_price.to_le_bytes());
        let unpacked = ConstantPriceCurve::unpack(&packed).unwrap();
        assert_eq!(curve, unpacked);
    }

    fn almost_equal(a: u128, b: u128) {
        if a >= b {
            assert!(a - b <= 1);
        } else {
            assert!(b - a <= 1);
        }
    }

    fn check_pool_token_conversion(
        token_b_price: u128,
        swap_token_a_amount: u128,
        swap_token_b_amount: u128,
        token_b_amount: u128,
    ) {
        let token_a_amount = token_b_amount * token_b_price;
        let curve = ConstantPriceCurve {
            token_b_price: token_b_price as u64,
        };
        let pool_supply = curve.new_pool_supply();
        let pool_tokens_from_a = curve
            .trading_tokens_to_pool_tokens(
                token_a_amount,
                swap_token_a_amount,
                swap_token_b_amount,
                pool_supply,
                TradeDirection::AtoB,
            )
            .unwrap();
        let pool_tokens_from_b = curve
            .trading_tokens_to_pool_tokens(
                token_b_amount,
                swap_token_a_amount,
                swap_token_b_amount,
                pool_supply,
                TradeDirection::BtoA,
            )
            .unwrap();
        let results = curve
            .pool_tokens_to_trading_tokens(
                pool_tokens_from_a + pool_tokens_from_b,
                pool_supply,
                swap_token_a_amount,
                swap_token_b_amount,
            )
            .unwrap();
        almost_equal(
            results.token_a_amount / token_b_price,
            token_a_amount / token_b_price,
        ); // takes care of truncation issues
        almost_equal(results.token_b_amount, token_b_amount);
    }

    #[test]
    fn pool_token_conversion() {
        let tests: &[(u128, u128, u128, u128)] = &[
            (10_000, 1_000_000, 1, 10),
            (10, 1_000, 100, 1),
            (1_251, 30, 1_288, 1_225),
            (1_000_251, 0, 1_288, 1),
            (1_000_000_000_000, 212, 10_000, 1),
        ];
        for (token_b_price, swap_token_a_amount, swap_token_b_amount, token_b_amount) in
            tests.iter()
        {
            check_pool_token_conversion(
                *token_b_price,
                *swap_token_a_amount,
                *swap_token_b_amount,
                *token_b_amount,
            );
        }
    }

    #[test]
    fn swap_calculation_large_price() {
        let token_b_price = 1123513u128;
        let curve = ConstantPriceCurve {
            token_b_price: token_b_price as u64,
        };
        let token_b_amount = 500u128;
        let token_a_amount = token_b_amount * token_b_price;
        let bad_result = curve.swap_without_fees(
            token_b_price - 1u128,
            token_a_amount,
            token_b_amount,
            TradeDirection::AtoB,
        );
        assert!(bad_result.is_none());
        let bad_result =
            curve.swap_without_fees(1u128, token_a_amount, token_b_amount, TradeDirection::AtoB);
        assert!(bad_result.is_none());
        let result = curve
            .swap_without_fees(
                token_b_price,
                token_a_amount,
                token_b_amount,
                TradeDirection::AtoB,
            )
            .unwrap();
        assert_eq!(result.source_amount_swapped, token_b_price);
        assert_eq!(result.destination_amount_swapped, 1u128);
    }

    #[test]
    fn swap_calculation_max_min() {
        let token_b_price = u64::MAX as u128;
        let curve = ConstantPriceCurve {
            token_b_price: token_b_price as u64,
        };
        let token_b_amount = 1u128;
        let token_a_amount = token_b_price;
        let bad_result = curve.swap_without_fees(
            token_b_price - 1u128,
            token_a_amount,
            token_b_amount,
            TradeDirection::AtoB,
        );
        assert!(bad_result.is_none());
        let bad_result =
            curve.swap_without_fees(1u128, token_a_amount, token_b_amount, TradeDirection::AtoB);
        assert!(bad_result.is_none());
        let bad_result =
            curve.swap_without_fees(0u128, token_a_amount, token_b_amount, TradeDirection::AtoB);
        assert!(bad_result.is_none());
        let result = curve
            .swap_without_fees(
                token_b_price,
                token_a_amount,
                token_b_amount,
                TradeDirection::AtoB,
            )
            .unwrap();
        assert_eq!(result.source_amount_swapped, token_b_price);
        assert_eq!(result.destination_amount_swapped, 1u128);
    }
}
