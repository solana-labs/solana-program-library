//! Swap calculations and curve implementations

use solana_sdk::{
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
};

use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use std::convert::{TryFrom, TryInto};
use std::fmt::Debug;

/// Initial amount of pool tokens for swap contract, hard-coded to something
/// "sensible" given a maximum of u64.
/// Note that on Ethereum, Uniswap uses the geometric mean of all provided
/// input amounts, and Balancer uses 100 * 10 ^ 18.
pub const INITIAL_SWAP_POOL_AMOUNT: u64 = 1_000_000_000;

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
        source_amount: u64,
        swap_source_amount: u64,
        swap_destination_amount: u64,
    ) -> Option<SwapResult>;

    /// Get the supply of a new pool (can be a default amount or calculated
    /// based on parameters)
    fn new_pool_supply(&self) -> u64;

    /// Get the amount of liquidity tokens for pool tokens given the total amount
    /// of liquidity tokens in the pool
    fn liquidity_tokens(
        &self,
        pool_tokens: u64,
        pool_token_supply: u64,
        total_liquidity_tokens: u64,
    ) -> Option<u64>;
}

/// Encodes all results of swapping from a source token to a destination token
pub struct SwapResult {
    /// New amount of source token
    pub new_source_amount: u64,
    /// New amount of destination token
    pub new_destination_amount: u64,
    /// Amount of destination token swapped
    pub amount_swapped: u64,
}

/// Helper function for mapping to SwapError::CalculationFailure
fn map_zero_to_none(x: u64) -> Option<u64> {
    if x == 0 {
        None
    } else {
        Some(x)
    }
}

/// Simple constant 1:1 swap curve, example of different swap curve implementations
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FlatCurve {
    /// Fee numerator
    pub fee_numerator: u64,
    /// Fee denominator
    pub fee_denominator: u64,
}

impl CurveCalculator for FlatCurve {
    /// Flat curve swap always returns 1:1 (minus fee)
    fn swap(
        &self,
        source_amount: u64,
        swap_source_amount: u64,
        swap_destination_amount: u64,
    ) -> Option<SwapResult> {
        // debit the fee to calculate the amount swapped
        let mut fee = source_amount
            .checked_mul(self.fee_numerator)?
            .checked_div(self.fee_denominator)?;
        if fee == 0 {
            fee = 1; // minimum fee of one token
        }

        let amount_swapped = source_amount.checked_sub(fee)?;
        let new_destination_amount = swap_destination_amount.checked_sub(amount_swapped)?;

        // actually add the whole amount coming in
        let new_source_amount = swap_source_amount.checked_add(source_amount)?;
        Some(SwapResult {
            new_source_amount,
            new_destination_amount,
            amount_swapped,
        })
    }

    /// Balancer-style fixed initial supply
    fn new_pool_supply(&self) -> u64 {
        INITIAL_SWAP_POOL_AMOUNT
    }

    /// Simple ratio calculation for how many liquidity tokens correspond to
    /// a certain number of pool tokens
    fn liquidity_tokens(
        &self,
        pool_tokens: u64,
        pool_token_supply: u64,
        total_liquidity_tokens: u64,
    ) -> Option<u64> {
        pool_tokens
            .checked_mul(total_liquidity_tokens)?
            .checked_div(pool_token_supply)
            .and_then(map_zero_to_none)
    }
}

/// IsInitialized is required to use `Pack::pack` and `Pack::unpack`
impl IsInitialized for FlatCurve {
    fn is_initialized(&self) -> bool {
        true
    }
}
impl Sealed for FlatCurve {}
impl Pack for FlatCurve {
    const LEN: usize = 16;
    /// Unpacks a byte buffer into a SwapCurve
    fn unpack_from_slice(input: &[u8]) -> Result<FlatCurve, ProgramError> {
        let input = array_ref![input, 0, 16];
        #[allow(clippy::ptr_offset_with_cast)]
        let (fee_numerator, fee_denominator) = array_refs![input, 8, 8];
        Ok(Self {
            fee_numerator: u64::from_le_bytes(*fee_numerator),
            fee_denominator: u64::from_le_bytes(*fee_denominator),
        })
    }

    fn pack_into_slice(&self, output: &mut [u8]) {
        (self as &dyn DynPack).pack_into_slice(output);
    }
}

impl DynPack for FlatCurve {
    fn pack_into_slice(&self, output: &mut [u8]) {
        let output = array_mut_ref![output, 0, 16];
        let (fee_numerator, fee_denominator) = mut_array_refs![output, 8, 8];
        *fee_numerator = self.fee_numerator.to_le_bytes();
        *fee_denominator = self.fee_denominator.to_le_bytes();
    }
}

/// The Uniswap invariant calculator.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ConstantProductCurve {
    /// Fee numerator
    pub fee_numerator: u64,
    /// Fee denominator
    pub fee_denominator: u64,
}

impl CurveCalculator for ConstantProductCurve {
    /// Constant product swap ensures x * y = constant
    fn swap(
        &self,
        source_amount: u64,
        swap_source_amount: u64,
        swap_destination_amount: u64,
    ) -> Option<SwapResult> {
        let invariant = swap_source_amount.checked_mul(swap_destination_amount)?;

        // debit the fee to calculate the amount swapped
        let mut fee = source_amount
            .checked_mul(self.fee_numerator)?
            .checked_div(self.fee_denominator)?;
        if fee == 0 {
            fee = 1; // minimum fee of one token
        }
        let new_source_amount_less_fee = swap_source_amount
            .checked_add(source_amount)?
            .checked_sub(fee)?;
        let new_destination_amount = invariant.checked_div(new_source_amount_less_fee)?;
        let amount_swapped =
            map_zero_to_none(swap_destination_amount.checked_sub(new_destination_amount)?)?;

        // actually add the whole amount coming in
        let new_source_amount = swap_source_amount.checked_add(source_amount)?;
        Some(SwapResult {
            new_source_amount,
            new_destination_amount,
            amount_swapped,
        })
    }

    /// Balancer-style supply starts at a constant.  This could be modified to
    /// follow the geometric mean, as done in Uniswap v2.
    fn new_pool_supply(&self) -> u64 {
        INITIAL_SWAP_POOL_AMOUNT
    }

    /// Simple ratio calculation to get the amount of liquidity tokens given
    /// pool information
    fn liquidity_tokens(
        &self,
        pool_tokens: u64,
        pool_token_supply: u64,
        total_liquidity_tokens: u64,
    ) -> Option<u64> {
        pool_tokens
            .checked_mul(total_liquidity_tokens)?
            .checked_div(pool_token_supply)
            .and_then(map_zero_to_none)
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
    const LEN: usize = 16;
    fn unpack_from_slice(input: &[u8]) -> Result<ConstantProductCurve, ProgramError> {
        let input = array_ref![input, 0, 16];
        #[allow(clippy::ptr_offset_with_cast)]
        let (fee_numerator, fee_denominator) = array_refs![input, 8, 8];
        Ok(Self {
            fee_numerator: u64::from_le_bytes(*fee_numerator),
            fee_denominator: u64::from_le_bytes(*fee_denominator),
        })
    }

    fn pack_into_slice(&self, output: &mut [u8]) {
        (self as &dyn DynPack).pack_into_slice(output);
    }
}

impl DynPack for ConstantProductCurve {
    fn pack_into_slice(&self, output: &mut [u8]) {
        let output = array_mut_ref![output, 0, 16];
        let (fee_numerator, fee_denominator) = mut_array_refs![output, 8, 8];
        *fee_numerator = self.fee_numerator.to_le_bytes();
        *fee_denominator = self.fee_denominator.to_le_bytes();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_pool_amount() {
        let fee_numerator = 0;
        let fee_denominator = 1;
        let calculator = ConstantProductCurve {
            fee_numerator,
            fee_denominator,
        };
        assert_eq!(calculator.new_pool_supply(), INITIAL_SWAP_POOL_AMOUNT);
    }

    fn check_liquidity_pool_token_rate(
        token_a: u64,
        deposit: u64,
        supply: u64,
        expected: Option<u64>,
    ) {
        let fee_numerator = 0;
        let fee_denominator = 1;
        let calculator = ConstantProductCurve {
            fee_numerator,
            fee_denominator,
        };
        assert_eq!(
            calculator.liquidity_tokens(deposit, supply, token_a),
            expected
        );
    }

    #[test]
    fn issued_tokens() {
        check_liquidity_pool_token_rate(2, 5, 10, Some(1));
        check_liquidity_pool_token_rate(10, 5, 10, Some(5));
        check_liquidity_pool_token_rate(5, 5, 10, Some(2));
        check_liquidity_pool_token_rate(5, 5, 10, Some(2));
        check_liquidity_pool_token_rate(u64::MAX, 5, 10, None);
    }

    #[test]
    fn constant_product_swap_calculation() {
        // calculation on https://github.com/solana-labs/solana-program-library/issues/341
        let swap_source_amount: u64 = 1000;
        let swap_destination_amount: u64 = 50000;
        let fee_numerator: u64 = 1;
        let fee_denominator: u64 = 100;
        let source_amount: u64 = 100;
        let curve = ConstantProductCurve {
            fee_numerator,
            fee_denominator,
        };
        let result = curve
            .swap(source_amount, swap_source_amount, swap_destination_amount)
            .unwrap();
        assert_eq!(result.new_source_amount, 1100);
        assert_eq!(result.amount_swapped, 4505);
        assert_eq!(result.new_destination_amount, 45495);
    }

    #[test]
    fn flat_swap_calculation() {
        let swap_source_amount: u64 = 1000;
        let swap_destination_amount: u64 = 50000;
        let fee_numerator: u64 = 1;
        let fee_denominator: u64 = 100;
        let source_amount: u64 = 100;
        let curve = FlatCurve {
            fee_numerator,
            fee_denominator,
        };
        let result = curve
            .swap(source_amount, swap_source_amount, swap_destination_amount)
            .unwrap();
        let amount_swapped = 99;
        assert_eq!(result.new_source_amount, 1100);
        assert_eq!(result.amount_swapped, amount_swapped);
        assert_eq!(
            result.new_destination_amount,
            swap_destination_amount - amount_swapped
        );
    }

    #[test]
    fn pack_flat_curve() {
        let fee_numerator = 1;
        let fee_denominator = 4;
        let curve = FlatCurve {
            fee_numerator,
            fee_denominator,
        };

        let mut packed = [0u8; FlatCurve::LEN];
        Pack::pack_into_slice(&curve, &mut packed[..]);
        let unpacked = FlatCurve::unpack(&packed).unwrap();
        assert_eq!(curve, unpacked);

        let mut packed = vec![];
        packed.extend_from_slice(&fee_numerator.to_le_bytes());
        packed.extend_from_slice(&fee_denominator.to_le_bytes());
        let unpacked = FlatCurve::unpack(&packed).unwrap();
        assert_eq!(curve, unpacked);
    }

    #[test]
    fn pack_constant_product_curve() {
        let fee_numerator = 1;
        let fee_denominator = 4;
        let curve = ConstantProductCurve {
            fee_numerator,
            fee_denominator,
        };

        let mut packed = [0u8; ConstantProductCurve::LEN];
        Pack::pack_into_slice(&curve, &mut packed[..]);
        let unpacked = ConstantProductCurve::unpack(&packed).unwrap();
        assert_eq!(curve, unpacked);

        let mut packed = vec![];
        packed.extend_from_slice(&fee_numerator.to_le_bytes());
        packed.extend_from_slice(&fee_denominator.to_le_bytes());
        let unpacked = ConstantProductCurve::unpack(&packed).unwrap();
        assert_eq!(curve, unpacked);
    }

    #[test]
    fn pack_swap_curve() {
        let fee_numerator = 1;
        let fee_denominator = 4;
        let curve = ConstantProductCurve {
            fee_numerator,
            fee_denominator,
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
        packed.extend_from_slice(&fee_numerator.to_le_bytes());
        packed.extend_from_slice(&fee_denominator.to_le_bytes());
        packed.extend_from_slice(&[0u8; 48]); // padding
        let unpacked = SwapCurve::unpack_from_slice(&packed).unwrap();
        assert_eq!(swap_curve, unpacked);
    }
}
