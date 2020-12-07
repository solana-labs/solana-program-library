//! Simple constant price swap curve, set at init

use crate::{
    curve::calculator::{CurveCalculator, DynPack, SwapWithoutFeesResult, TradeDirection},
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
}
