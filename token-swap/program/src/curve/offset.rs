//! The Uniswap invariant calculator with an extra offset

use crate::{
    curve::{
        calculator::{
            CurveCalculator, DynPack, SwapWithoutFeesResult, TradeDirection, TradingTokenResult,
        },
        constant_product::swap,
        math::PreciseNumber,
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
    /// Constant product swap ensures token a * (token b + offset) = constant
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

    /// The conversion for the offset curve needs to take into account the
    /// offset
    fn pool_tokens_to_trading_tokens(
        &self,
        pool_tokens: u128,
        pool_token_supply: u128,
        swap_token_a_amount: u128,
        swap_token_b_amount: u128,
    ) -> Option<TradingTokenResult> {
        let token_b_offset = self.token_b_offset as u128;
        let token_a_amount = pool_tokens
            .checked_mul(swap_token_a_amount)?
            .checked_div(pool_token_supply)?;
        let token_b_amount = pool_tokens
            .checked_mul(swap_token_b_amount.checked_add(token_b_offset)?)?
            .checked_div(pool_token_supply)?;
        Some(TradingTokenResult {
            token_a_amount,
            token_b_amount,
        })
    }

    /// Get the amount of pool tokens for the given amount of token A and B,
    /// taking into account the offset
    fn trading_tokens_to_pool_tokens(
        &self,
        source_amount: u128,
        swap_token_a_amount: u128,
        swap_token_b_amount: u128,
        pool_supply: u128,
        trade_direction: TradeDirection,
    ) -> Option<u128> {
        let token_b_offset = self.token_b_offset as u128;
        let swap_source_amount = match trade_direction {
            TradeDirection::AtoB => swap_token_a_amount,
            TradeDirection::BtoA => swap_token_b_amount.checked_add(token_b_offset)?,
        };
        let swap_source_amount = PreciseNumber::new(swap_source_amount)?;
        let source_amount = PreciseNumber::new(source_amount)?;
        let ratio = source_amount.checked_div(&swap_source_amount)?;
        let one = PreciseNumber::new(1)?;
        let two = PreciseNumber::new(2)?;
        let base = one.checked_add(&ratio)?;
        let guess = base.checked_div(&two)?;
        let root = base
            .newtonian_root_approximation(&two, guess)?
            .checked_sub(&one)?;
        let pool_supply = PreciseNumber::new(pool_supply)?;
        pool_supply.checked_mul(&root)?.to_imprecise()
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

    /// Offset curves can cause arbitrage opportunities if outside users are
    /// allowed to deposit.  For example, in the offset curve, if there's swap
    /// with 1 million of token A against an offset of 2 million token B,
    /// someone else can deposit 1 million A and 2 million B for LP tokens.
    /// The pool creator can then use their LP tokens to steal the 2 million B,
    fn allows_deposits(&self) -> bool {
        false
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
    use crate::curve::calculator::test::check_pool_token_conversion;

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

    #[test]
    fn pool_token_conversion() {
        let tests: &[(u64, u128, u128, u128)] = &[
            (10_000, 1_000_000, 1, 100_000),
            (10, 1_000, 100, 100),
            (1_251, 30, 1_288, 100_000),
            (1_000_251, 1_000, 1_288, 100_000),
            (1_000_000_000_000, 212, 10_000, 100_000),
        ];
        for (token_b_offset, swap_token_a_amount, swap_token_b_amount, token_a_amount) in
            tests.iter()
        {
            let curve = OffsetCurve {
                token_b_offset: *token_b_offset,
            };
            check_pool_token_conversion(
                &curve,
                *swap_token_a_amount,
                *swap_token_b_amount,
                *token_a_amount,
            );
        }
    }
}
