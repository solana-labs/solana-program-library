//! Base curve implementation

use solana_program::{
    program_error::ProgramError,
    program_pack::{Pack, Sealed},
};

use crate::curve::{
    calculator::CurveCalculator, constant_product::ConstantProductCurve, flat::FlatCurve,
    stable::StableCurve,
};
use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use std::convert::{TryFrom, TryInto};
use std::fmt::Debug;

/// Curve types supported by the token-swap program.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CurveType {
    /// Uniswap-style constant product curve, invariant = token_a_amount * token_b_amount
    ConstantProduct,
    /// Flat line, always providing 1:1 from one token to another
    Flat,
    /// Stable, Like uniswap, but with wide zone of 1:1 instead of one point
    Stable,
}

/// Concrete struct to wrap around the trait object which performs calculation.
#[repr(C)]
#[derive(Debug)]
pub struct SwapCurve {
    /// The type of curve contained in the calculator, helpful for outside
    /// queries
    pub curve_type: CurveType,
    /// The actual calculator, represented as a trait object to allow for many
    /// different types of curves
    pub calculator: Box<dyn CurveCalculator>,
}

/// Default implementation for SwapCurve cannot be derived because of
/// the contained Box.
impl Default for SwapCurve {
    fn default() -> Self {
        let curve_type: CurveType = Default::default();
        let calculator: ConstantProductCurve = Default::default();
        Self {
            curve_type,
            calculator: Box::new(calculator),
        }
    }
}

/// Clone takes advantage of pack / unpack to get around the difficulty of
/// cloning dynamic objects.
/// Note that this is only to be used for testing.
#[cfg(any(test, feature = "fuzz"))]
impl Clone for SwapCurve {
    fn clone(&self) -> Self {
        let mut packed_self = [0u8; Self::LEN];
        Self::pack_into_slice(self, &mut packed_self);
        Self::unpack_from_slice(&packed_self).unwrap()
    }
}

/// Simple implementation for PartialEq which assumes that the output of
/// `Pack` is enough to guarantee equality
impl PartialEq for SwapCurve {
    fn eq(&self, other: &Self) -> bool {
        let mut packed_self = [0u8; Self::LEN];
        Self::pack_into_slice(self, &mut packed_self);
        let mut packed_other = [0u8; Self::LEN];
        Self::pack_into_slice(other, &mut packed_other);
        packed_self[..] == packed_other[..]
    }
}

impl Sealed for SwapCurve {}
impl Pack for SwapCurve {
    /// Size of encoding of all curve parameters, which include fees and any other
    /// constants used to calculate swaps, deposits, and withdrawals.
    /// This includes 1 byte for the type, and 72 for the calculator to use as
    /// it needs.  Some calculators may be smaller than 72 bytes.
    const LEN: usize = 73;

    /// Unpacks a byte buffer into a SwapCurve
    fn unpack_from_slice(input: &[u8]) -> Result<Self, ProgramError> {
        let input = array_ref![input, 0, 73];
        #[allow(clippy::ptr_offset_with_cast)]
        let (curve_type, calculator) = array_refs![input, 1, 72];
        let curve_type = curve_type[0].try_into()?;
        Ok(Self {
            curve_type,
            calculator: match curve_type {
                CurveType::ConstantProduct => {
                    Box::new(ConstantProductCurve::unpack_from_slice(calculator)?)
                }
                CurveType::Flat => Box::new(FlatCurve::unpack_from_slice(calculator)?),
                CurveType::Stable => Box::new(StableCurve::unpack_from_slice(calculator)?),
            },
        })
    }

    /// Pack SwapCurve into a byte buffer
    fn pack_into_slice(&self, output: &mut [u8]) {
        let output = array_mut_ref![output, 0, 73];
        let (curve_type, calculator) = mut_array_refs![output, 1, 72];
        curve_type[0] = self.curve_type as u8;
        self.calculator.pack_into_slice(&mut calculator[..]);
    }
}

/// Sensible default of CurveType to ConstantProduct, the most popular and
/// well-known curve type.
impl Default for CurveType {
    fn default() -> Self {
        CurveType::ConstantProduct
    }
}

impl TryFrom<u8> for CurveType {
    type Error = ProgramError;

    fn try_from(curve_type: u8) -> Result<Self, Self::Error> {
        match curve_type {
            0 => Ok(CurveType::ConstantProduct),
            1 => Ok(CurveType::Flat),
            2 => Ok(CurveType::Stable),
            _ => Err(ProgramError::InvalidAccountData),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_swap_curve() {
        let trade_fee_numerator = 1;
        let trade_fee_denominator = 4;
        let owner_trade_fee_numerator = 2;
        let owner_trade_fee_denominator = 5;
        let owner_withdraw_fee_numerator = 4;
        let owner_withdraw_fee_denominator = 10;
        let host_fee_numerator = 7;
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
        let curve_type = CurveType::ConstantProduct;
        let swap_curve = SwapCurve {
            curve_type,
            calculator: Box::new(curve),
        };

        let mut packed = [0u8; SwapCurve::LEN];
        Pack::pack_into_slice(&swap_curve, &mut packed[..]);
        let unpacked = SwapCurve::unpack_from_slice(&packed).unwrap();
        assert_eq!(swap_curve, unpacked);

        let mut packed = vec![];
        packed.push(curve_type as u8);
        packed.extend_from_slice(&trade_fee_numerator.to_le_bytes());
        packed.extend_from_slice(&trade_fee_denominator.to_le_bytes());
        packed.extend_from_slice(&owner_trade_fee_numerator.to_le_bytes());
        packed.extend_from_slice(&owner_trade_fee_denominator.to_le_bytes());
        packed.extend_from_slice(&owner_withdraw_fee_numerator.to_le_bytes());
        packed.extend_from_slice(&owner_withdraw_fee_denominator.to_le_bytes());
        packed.extend_from_slice(&host_fee_numerator.to_le_bytes());
        packed.extend_from_slice(&host_fee_denominator.to_le_bytes());
        packed.extend_from_slice(&0u64.to_le_bytes()); // 8 bytes reserved for larget calcs
        let unpacked = SwapCurve::unpack_from_slice(&packed).unwrap();
        assert_eq!(swap_curve, unpacked);
    }
}
