//! Various constraints as required for production environments

use crate::{
    curve::{
        base::{CurveType, SwapCurve},
        fees::Fees,
    },
    error::SwapError,
};
use solana_program::program_error::ProgramError;

#[cfg(feature = "production")]
use std::env;

/// Encodes fee constraints, used in multihost environments where the program
/// may be used by multiple frontends, to ensure that proper fees are being
/// assessed.
/// Since this struct needs to be created at compile-time, we only have access
/// to const functions and constructors. Since SwapCurve contains a Arc, it
/// cannot be used, so we have to split the curves based on their types.
pub struct SwapConstraints<'a> {
    /// Owner of the program
    pub owner_key: &'a str,
    /// Valid curve types
    pub valid_curve_types: &'a [CurveType],
    /// Valid fees
    pub fees: &'a Fees,
}

impl<'a> SwapConstraints<'a> {
    /// Checks that the provided curve is valid for the given constraints
    pub fn validate_curve(&self, swap_curve: &SwapCurve) -> Result<(), ProgramError> {
        if self
            .valid_curve_types
            .iter()
            .any(|x| *x == swap_curve.curve_type)
        {
            Ok(())
        } else {
            Err(SwapError::UnsupportedCurveType.into())
        }
    }

    /// Checks that the provided curve is valid for the given constraints
    pub fn validate_fees(&self, fees: &Fees) -> Result<(), ProgramError> {
        if fees.trade_fee_numerator >= self.fees.trade_fee_numerator
            && fees.trade_fee_denominator == self.fees.trade_fee_denominator
            && fees.owner_trade_fee_numerator >= self.fees.owner_trade_fee_numerator
            && fees.owner_trade_fee_denominator == self.fees.owner_trade_fee_denominator
            && fees.owner_withdraw_fee_numerator >= self.fees.owner_withdraw_fee_numerator
            && fees.owner_withdraw_fee_denominator == self.fees.owner_withdraw_fee_denominator
            && fees.host_fee_numerator == self.fees.host_fee_numerator
            && fees.host_fee_denominator == self.fees.host_fee_denominator
        {
            Ok(())
        } else {
            Err(SwapError::InvalidFee.into())
        }
    }
}

#[cfg(feature = "production")]
const OWNER_KEY: &str = env!("SWAP_PROGRAM_OWNER_FEE_ADDRESS");
#[cfg(feature = "production")]
const FEES: &Fees = &Fees {
    trade_fee_numerator: 0,
    trade_fee_denominator: 10000,
    owner_trade_fee_numerator: 5,
    owner_trade_fee_denominator: 10000,
    owner_withdraw_fee_numerator: 0,
    owner_withdraw_fee_denominator: 0,
    host_fee_numerator: 20,
    host_fee_denominator: 100,
};
#[cfg(feature = "production")]
const VALID_CURVE_TYPES: &[CurveType] = &[CurveType::ConstantPrice, CurveType::ConstantProduct];

/// Fee structure defined by program creator in order to enforce certain
/// fees when others use the program.  Adds checks on pool creation and
/// swapping to ensure the correct fees and account owners are passed.
/// Fees provided during production build currently are considered min
/// fees that creator of the pool can specify. Host fee is a fixed
/// percentage that host receives as a portion of owner fees
pub const SWAP_CONSTRAINTS: Option<SwapConstraints> = {
    #[cfg(feature = "production")]
    {
        Some(SwapConstraints {
            owner_key: OWNER_KEY,
            valid_curve_types: VALID_CURVE_TYPES,
            fees: FEES,
        })
    }
    #[cfg(not(feature = "production"))]
    {
        None
    }
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::curve::{base::CurveType, constant_product::ConstantProductCurve};
    use std::sync::Arc;

    #[test]
    fn validate_fees() {
        let trade_fee_numerator = 1;
        let trade_fee_denominator = 4;
        let owner_trade_fee_numerator = 2;
        let owner_trade_fee_denominator = 5;
        let owner_withdraw_fee_numerator = 4;
        let owner_withdraw_fee_denominator = 10;
        let host_fee_numerator = 10;
        let host_fee_denominator = 100;
        let owner_key = "";
        let curve_type = CurveType::ConstantProduct;
        let valid_fees = Fees {
            trade_fee_numerator,
            trade_fee_denominator,
            owner_trade_fee_numerator,
            owner_trade_fee_denominator,
            owner_withdraw_fee_numerator,
            owner_withdraw_fee_denominator,
            host_fee_numerator,
            host_fee_denominator,
        };
        let calculator = ConstantProductCurve {};
        let swap_curve = SwapCurve {
            curve_type,
            calculator: Arc::new(calculator.clone()),
        };
        let constraints = SwapConstraints {
            owner_key,
            valid_curve_types: &[curve_type],
            fees: &valid_fees,
        };

        constraints.validate_curve(&swap_curve).unwrap();
        constraints.validate_fees(&valid_fees).unwrap();

        let mut fees = valid_fees.clone();
        fees.trade_fee_numerator = trade_fee_numerator - 1;
        assert_eq!(
            Err(SwapError::InvalidFee.into()),
            constraints.validate_fees(&fees),
        );
        fees.trade_fee_numerator = trade_fee_numerator;

        // passing higher fee is ok
        fees.trade_fee_numerator = trade_fee_numerator - 1;
        assert_eq!(constraints.validate_fees(&valid_fees), Ok(()));
        fees.trade_fee_numerator = trade_fee_numerator;

        fees.trade_fee_denominator = trade_fee_denominator - 1;
        assert_eq!(
            Err(SwapError::InvalidFee.into()),
            constraints.validate_fees(&fees),
        );
        fees.trade_fee_denominator = trade_fee_denominator;

        fees.trade_fee_denominator = trade_fee_denominator + 1;
        assert_eq!(
            Err(SwapError::InvalidFee.into()),
            constraints.validate_fees(&fees),
        );
        fees.trade_fee_denominator = trade_fee_denominator;

        fees.owner_trade_fee_numerator = owner_trade_fee_numerator - 1;
        assert_eq!(
            Err(SwapError::InvalidFee.into()),
            constraints.validate_fees(&fees),
        );
        fees.owner_trade_fee_numerator = owner_trade_fee_numerator;

        // passing higher fee is ok
        fees.owner_trade_fee_numerator = owner_trade_fee_numerator - 1;
        assert_eq!(constraints.validate_fees(&valid_fees), Ok(()));
        fees.owner_trade_fee_numerator = owner_trade_fee_numerator;

        fees.owner_trade_fee_denominator = owner_trade_fee_denominator - 1;
        assert_eq!(
            Err(SwapError::InvalidFee.into()),
            constraints.validate_fees(&fees),
        );
        fees.owner_trade_fee_denominator = owner_trade_fee_denominator;

        let swap_curve = SwapCurve {
            curve_type: CurveType::ConstantPrice,
            calculator: Arc::new(calculator),
        };
        assert_eq!(
            Err(SwapError::UnsupportedCurveType.into()),
            constraints.validate_curve(&swap_curve),
        );
    }
}
