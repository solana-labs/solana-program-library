//! Simple constant price swap curve, set at init

use crate::{
    curve::calculator::{
        map_zero_to_none, CurveCalculator, DynPack, SwapWithoutFeesResult, TradeDirection,
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
    /// Constant price curve always returns 1 token B for `token_b_price`
    /// amount of token A
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

    fn validate(&self) -> Result<(), SwapError> {
        if self.token_b_price == 0 {
            Err(SwapError::InvalidCurve)
        } else {
            Ok(())
        }
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

    #[test]
    fn pack_flat_curve() {
        let token_b_price = 193871;
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
}
