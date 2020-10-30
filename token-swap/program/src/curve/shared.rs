//! Swap calculations and curve implementations

use solana_program::{
    program_error::ProgramError,
    program_pack::{Pack, Sealed},
};

use crate::curve::{cp::ConstantProductCurve, flat::FlatCurve};
use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use std::convert::{TryFrom, TryInto};
use std::fmt::Debug;

/// Initial amount of pool tokens for swap contract, hard-coded to something
/// "sensible" given a maximum of u128.
/// Note that on Ethereum, Uniswap uses the geometric mean of all provided
/// input amounts, and Balancer uses 100 * 10 ^ 18.
pub const INITIAL_SWAP_POOL_AMOUNT: u128 = 1_000_000_000;

/// Helper function for calcuating swap fee
pub fn calculate_fee(
    token_amount: u128,
    fee_numerator: u128,
    fee_denominator: u128,
) -> Option<u128> {
    if fee_numerator == 0 {
        Some(0)
    } else {
        let fee = token_amount
            .checked_mul(fee_numerator)?
            .checked_div(fee_denominator)?;
        if fee == 0 {
            Some(1) // minimum fee of one token
        } else {
            Some(fee)
        }
    }
}

/// Helper function for mapping to SwapError::CalculationFailure
pub fn map_zero_to_none(x: u128) -> Option<u128> {
    if x == 0 {
        None
    } else {
        Some(x)
    }
}

/// Curve types supported by the token-swap program.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CurveType {
    /// Uniswap-style constant product curve, invariant = token_a_amount * token_b_amount
    ConstantProduct,
    /// Flat line, always providing 1:1 from one token to another
    Flat,
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
#[cfg(test)]
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
    /// This includes 1 byte for the type, and 64 for the calculator to use as
    /// it needs.  Some calculators may be smaller than 64 bytes.
    const LEN: usize = 65;

    /// Unpacks a byte buffer into a SwapCurve
    fn unpack_from_slice(input: &[u8]) -> Result<Self, ProgramError> {
        let input = array_ref![input, 0, 65];
        #[allow(clippy::ptr_offset_with_cast)]
        let (curve_type, calculator) = array_refs![input, 1, 64];
        let curve_type = curve_type[0].try_into()?;
        Ok(Self {
            curve_type,
            calculator: match curve_type {
                CurveType::ConstantProduct => {
                    Box::new(ConstantProductCurve::unpack_from_slice(calculator)?)
                }
                CurveType::Flat => Box::new(FlatCurve::unpack_from_slice(calculator)?),
            },
        })
    }

    /// Pack SwapCurve into a byte buffer
    fn pack_into_slice(&self, output: &mut [u8]) {
        let output = array_mut_ref![output, 0, 65];
        let (curve_type, calculator) = mut_array_refs![output, 1, 64];
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
            _ => Err(ProgramError::InvalidAccountData),
        }
    }
}

/// Trait for packing of trait objects, required because structs that implement
/// `Pack` cannot be used as trait objects (as `dyn Pack`).
pub trait DynPack {
    /// Only required function is to pack given a trait object
    fn pack_into_slice(&self, dst: &mut [u8]);
}

/// Trait representing operations required on a swap curve
pub trait CurveCalculator: Debug + DynPack {
    /// Calculate how much destination token will be provided given an amount
    /// of source token.
    fn swap(
        &self,
        source_amount: u128,
        swap_source_amount: u128,
        swap_destination_amount: u128,
    ) -> Option<SwapResult>;

    /// Calculate the withdraw fee in pool tokens
    /// Default implementation assumes no fee
    fn owner_withdraw_fee(&self, _pool_tokens: u128) -> Option<u128> {
        Some(0)
    }

    /// Calculate the trading fee in trading tokens
    /// Default implementation assumes no fee
    fn trading_fee(&self, _trading_tokens: u128) -> Option<u128> {
        Some(0)
    }

    /// Calculate the pool token equivalent of the owner fee on trade
    /// See the math at: https://balancer.finance/whitepaper/#single-asset-deposit
    /// For the moment, we do an approximation for the square root.  For numbers
    /// just above 1, simply dividing by 2 brings you very close to the correct
    /// value.
    fn owner_fee_to_pool_tokens(
        &self,
        owner_fee: u128,
        trading_token_amount: u128,
        pool_supply: u128,
        tokens_in_pool: u128,
    ) -> Option<u128> {
        // Get the trading fee incurred if the owner fee is swapped for the other side
        let trade_fee = self.trading_fee(owner_fee)?;
        let owner_fee = owner_fee.checked_sub(trade_fee)?;
        pool_supply
            .checked_mul(owner_fee)?
            .checked_div(trading_token_amount)?
            .checked_div(tokens_in_pool)
    }

    /// Get the supply for a new pool
    /// The default implementation is a Balancer-style fixed initial supply
    fn new_pool_supply(&self) -> u128 {
        INITIAL_SWAP_POOL_AMOUNT
    }

    /// Get the amount of trading tokens for the given amount of pool tokens,
    /// provided the total trading tokens and supply of pool tokens.
    /// The default implementation is a simple ratio calculation for how many
    /// trading tokens correspond to a certain number of pool tokens
    fn pool_tokens_to_trading_tokens(
        &self,
        pool_tokens: u128,
        pool_token_supply: u128,
        total_trading_tokens: u128,
    ) -> Option<u128> {
        pool_tokens
            .checked_mul(total_trading_tokens)?
            .checked_div(pool_token_supply)
            .and_then(map_zero_to_none)
    }

    /// Calculate the host fee based on the owner fee, only used in production
    /// situations where a program is hosted by multiple frontends
    fn host_fee(&self, _owner_fee: u128) -> Option<u128> {
        Some(0)
    }
}

/// Encodes all results of swapping from a source token to a destination token
pub struct SwapResult {
    /// New amount of source token
    pub new_source_amount: u128,
    /// New amount of destination token
    pub new_destination_amount: u128,
    /// Amount of destination token swapped
    pub amount_swapped: u128,
    /// Amount of source tokens going to pool holders
    pub trade_fee: u128,
    /// Amount of source tokens going to owner
    pub owner_fee: u128,
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
        let unpacked = SwapCurve::unpack_from_slice(&packed).unwrap();
        assert_eq!(swap_curve, unpacked);
    }
}
