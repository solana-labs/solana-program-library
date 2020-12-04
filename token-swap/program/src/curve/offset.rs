//! The Uniswap invariant calculator with an extra offset

use crate::{
    curve::{
        calculator::{CurveCalculator, DynPack, SwapWithoutFeesResult, TradeDirection},
        constant_product::swap,
    },
    error::SwapError,
};
use arrayref::{array_mut_ref, array_ref};
use solana_program::{
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
};

/// Offset curve, uses ConstantProduct under the hood, but adds an offset to
/// one side on swap calculations
#[derive(Clone, Debug, Default, PartialEq)]
pub struct OffsetCurve {
    /// Amount to offset the token B liquidity account
    pub token_b_offset: u64,
}

impl CurveCalculator for OffsetCurve {
    /// Constant product swap ensures x * y = constant
    fn swap_without_fees(
        &self,
        source_amount: u128,
        swap_source_amount: u128,
        swap_destination_amount: u128,
        trade_direction: TradeDirection,
    ) -> Option<SwapWithoutFeesResult> {
        let token_b_offset = self.token_b_offset as u128;
        let swap_source_amount = match trade_direction {
            TradeDirection::AtoB => swap_source_amount,
            TradeDirection::BtoA => swap_source_amount.checked_add(token_b_offset)?,
        };
        let swap_destination_amount = match trade_direction {
            TradeDirection::AtoB => swap_destination_amount.checked_add(token_b_offset)?,
            TradeDirection::BtoA => swap_destination_amount,
        };
        swap(source_amount, swap_source_amount, swap_destination_amount)
    }

    fn validate(&self) -> Result<(), SwapError> {
        if self.token_b_offset == 0 {
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
impl IsInitialized for OffsetCurve {
    fn is_initialized(&self) -> bool {
        true
    }
}
impl Sealed for OffsetCurve {}
impl Pack for OffsetCurve {
    const LEN: usize = 8;
    fn pack_into_slice(&self, output: &mut [u8]) {
        (self as &dyn DynPack).pack_into_slice(output);
    }

    fn unpack_from_slice(input: &[u8]) -> Result<OffsetCurve, ProgramError> {
        let token_b_offset = array_ref![input, 0, 8];
        Ok(Self {
            token_b_offset: u64::from_le_bytes(*token_b_offset),
        })
    }
}

impl DynPack for OffsetCurve {
    fn pack_into_slice(&self, output: &mut [u8]) {
        let token_b_offset = array_mut_ref![output, 0, 8];
        *token_b_offset = self.token_b_offset.to_le_bytes();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_curve() {
        let token_b_offset = u64::MAX;
        let curve = OffsetCurve { token_b_offset };

        let mut packed = [0u8; OffsetCurve::LEN];
        Pack::pack_into_slice(&curve, &mut packed[..]);
        let unpacked = OffsetCurve::unpack(&packed).unwrap();
        assert_eq!(curve, unpacked);

        let mut packed = vec![];
        packed.extend_from_slice(&token_b_offset.to_le_bytes());
        let unpacked = OffsetCurve::unpack(&packed).unwrap();
        assert_eq!(curve, unpacked);
    }

    #[test]
    fn swap_no_offset() {
        let swap_source_amount: u128 = 1_000;
        let swap_destination_amount: u128 = 50_000;
        let source_amount: u128 = 100;
        let curve = OffsetCurve::default();
        let result = curve
            .swap_without_fees(
                source_amount,
                swap_source_amount,
                swap_destination_amount,
                TradeDirection::AtoB,
            )
            .unwrap();
        assert_eq!(result.source_amount_swapped, source_amount);
        assert_eq!(result.destination_amount_swapped, 4545);
        let result = curve
            .swap_without_fees(
                source_amount,
                swap_source_amount,
                swap_destination_amount,
                TradeDirection::BtoA,
            )
            .unwrap();
        assert_eq!(result.source_amount_swapped, source_amount);
        assert_eq!(result.destination_amount_swapped, 4545);
    }

    #[test]
    fn swap_offset() {
        let swap_source_amount: u128 = 1_000_000;
        let swap_destination_amount: u128 = 0;
        let source_amount: u128 = 100;
        let token_b_offset = 1_000_000;
        let curve = OffsetCurve { token_b_offset };
        let result = curve
            .swap_without_fees(
                source_amount,
                swap_source_amount,
                swap_destination_amount,
                TradeDirection::AtoB,
            )
            .unwrap();
        assert_eq!(result.source_amount_swapped, source_amount);
        assert_eq!(result.destination_amount_swapped, source_amount - 1);

        let bad_result = curve.swap_without_fees(
            source_amount,
            swap_source_amount,
            swap_destination_amount,
            TradeDirection::BtoA,
        );
        assert!(bad_result.is_none());
    }

    #[test]
    fn swap_a_to_b_max_offset() {
        let swap_source_amount: u128 = 10_000_000;
        let swap_destination_amount: u128 = 1_000;
        let source_amount: u128 = 1_000;
        let token_b_offset = u64::MAX;
        let curve = OffsetCurve { token_b_offset };
        let result = curve
            .swap_without_fees(
                source_amount,
                swap_source_amount,
                swap_destination_amount,
                TradeDirection::AtoB,
            )
            .unwrap();
        assert_eq!(result.source_amount_swapped, source_amount);
        assert_eq!(result.destination_amount_swapped, 1_844_489_958_375_117);
    }

    #[test]
    fn swap_b_to_a_max_offset() {
        let swap_source_amount: u128 = 10_000_000;
        let swap_destination_amount: u128 = 1_000;
        let source_amount: u128 = u64::MAX.into();
        let token_b_offset = u64::MAX;
        let curve = OffsetCurve { token_b_offset };
        let result = curve
            .swap_without_fees(
                source_amount,
                swap_source_amount,
                swap_destination_amount,
                TradeDirection::BtoA,
            )
            .unwrap();
        assert_eq!(result.source_amount_swapped, 18_373_104_376_818_475_561);
        assert_eq!(result.destination_amount_swapped, 499);
    }
}
