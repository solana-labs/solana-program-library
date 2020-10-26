//! Swap calculations and curve implementations

use solana_program::{
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
};

use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use std::convert::{TryFrom, TryInto};
use std::fmt::Debug;

/// Initial amount of pool tokens for swap contract, hard-coded to something
/// "sensible" given a maximum of u128.
/// Note that on Ethereum, Uniswap uses the geometric mean of all provided
/// input amounts, and Balancer uses 100 * 10 ^ 18.
pub const INITIAL_SWAP_POOL_AMOUNT: u128 = 1_000_000_000;

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

/// Helper function for mapping to SwapError::CalculationFailure
fn map_zero_to_none(x: u128) -> Option<u128> {
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
    pub trade_fee_numerator: u64,
    /// Fee denominator
    pub trade_fee_denominator: u64,
    /// Owner trade fee numerator
    pub owner_trade_fee_numerator: u64,
    /// Owner trade fee denominator
    pub owner_trade_fee_denominator: u64,
    /// Owner withdraw fee numerator
    pub owner_withdraw_fee_numerator: u64,
    /// Owner withdraw fee denominator
    pub owner_withdraw_fee_denominator: u64,
}

fn calculate_fee(token_amount: u128, fee_numerator: u128, fee_denominator: u128) -> Option<u128> {
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

impl CurveCalculator for FlatCurve {
    /// Flat curve swap always returns 1:1 (minus fee)
    fn swap(
        &self,
        source_amount: u128,
        swap_source_amount: u128,
        swap_destination_amount: u128,
    ) -> Option<SwapResult> {
        // debit the fee to calculate the amount swapped
        let trade_fee = calculate_fee(
            source_amount,
            u128::try_from(self.trade_fee_numerator).ok()?,
            u128::try_from(self.trade_fee_denominator).ok()?,
        )?;
        let owner_fee = calculate_fee(
            source_amount,
            u128::try_from(self.owner_trade_fee_numerator).ok()?,
            u128::try_from(self.owner_trade_fee_denominator).ok()?,
        )?;

        let amount_swapped = source_amount
            .checked_sub(trade_fee)?
            .checked_sub(owner_fee)?;
        let new_destination_amount = swap_destination_amount.checked_sub(amount_swapped)?;

        // actually add the whole amount coming in
        let new_source_amount = swap_source_amount.checked_add(source_amount)?;
        Some(SwapResult {
            new_source_amount,
            new_destination_amount,
            amount_swapped,
            trade_fee,
            owner_fee,
        })
    }

    /// Calculate the withdraw fee in pool tokens
    fn owner_withdraw_fee(&self, pool_tokens: u128) -> Option<u128> {
        calculate_fee(
            pool_tokens,
            u128::try_from(self.owner_withdraw_fee_numerator).ok()?,
            u128::try_from(self.owner_withdraw_fee_denominator).ok()?,
        )
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
    const LEN: usize = 48;
    fn unpack_from_slice(input: &[u8]) -> Result<FlatCurve, ProgramError> {
        let input = array_ref![input, 0, 48];
        #[allow(clippy::ptr_offset_with_cast)]
        let (
            trade_fee_numerator,
            trade_fee_denominator,
            owner_trade_fee_numerator,
            owner_trade_fee_denominator,
            owner_withdraw_fee_numerator,
            owner_withdraw_fee_denominator,
        ) = array_refs![input, 8, 8, 8, 8, 8, 8];
        Ok(Self {
            trade_fee_numerator: u64::from_le_bytes(*trade_fee_numerator),
            trade_fee_denominator: u64::from_le_bytes(*trade_fee_denominator),
            owner_trade_fee_numerator: u64::from_le_bytes(*owner_trade_fee_numerator),
            owner_trade_fee_denominator: u64::from_le_bytes(*owner_trade_fee_denominator),
            owner_withdraw_fee_numerator: u64::from_le_bytes(*owner_withdraw_fee_numerator),
            owner_withdraw_fee_denominator: u64::from_le_bytes(*owner_withdraw_fee_denominator),
        })
    }

    fn pack_into_slice(&self, output: &mut [u8]) {
        (self as &dyn DynPack).pack_into_slice(output);
    }
}

impl DynPack for FlatCurve {
    fn pack_into_slice(&self, output: &mut [u8]) {
        let output = array_mut_ref![output, 0, 48];
        let (
            trade_fee_numerator,
            trade_fee_denominator,
            owner_trade_fee_numerator,
            owner_trade_fee_denominator,
            owner_withdraw_fee_numerator,
            owner_withdraw_fee_denominator,
        ) = mut_array_refs![output, 8, 8, 8, 8, 8, 8];
        *trade_fee_numerator = self.trade_fee_numerator.to_le_bytes();
        *trade_fee_denominator = self.trade_fee_denominator.to_le_bytes();
        *owner_trade_fee_numerator = self.owner_trade_fee_numerator.to_le_bytes();
        *owner_trade_fee_denominator = self.owner_trade_fee_denominator.to_le_bytes();
        *owner_withdraw_fee_numerator = self.owner_withdraw_fee_numerator.to_le_bytes();
        *owner_withdraw_fee_denominator = self.owner_withdraw_fee_denominator.to_le_bytes();
    }
}

/// The Uniswap invariant calculator.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ConstantProductCurve {
    /// Trade fee numerator
    pub trade_fee_numerator: u64,
    /// Trade fee denominator
    pub trade_fee_denominator: u64,
    /// Owner trade fee numerator
    pub owner_trade_fee_numerator: u64,
    /// Owner trade fee denominator
    pub owner_trade_fee_denominator: u64,
    /// Owner withdraw fee numerator
    pub owner_withdraw_fee_numerator: u64,
    /// Owner withdraw fee denominator
    pub owner_withdraw_fee_denominator: u64,
}

impl CurveCalculator for ConstantProductCurve {
    /// Constant product swap ensures x * y = constant
    fn swap(
        &self,
        source_amount: u128,
        swap_source_amount: u128,
        swap_destination_amount: u128,
    ) -> Option<SwapResult> {
        // debit the fee to calculate the amount swapped
        let trade_fee = self.trading_fee(source_amount)?;
        let owner_fee = calculate_fee(
            source_amount,
            u128::try_from(self.owner_trade_fee_numerator).ok()?,
            u128::try_from(self.owner_trade_fee_denominator).ok()?,
        )?;

        let invariant = swap_source_amount.checked_mul(swap_destination_amount)?;
        let new_source_amount_less_fee = swap_source_amount
            .checked_add(source_amount)?
            .checked_sub(trade_fee)?
            .checked_sub(owner_fee)?;
        let new_destination_amount = invariant.checked_div(new_source_amount_less_fee)?;
        let amount_swapped =
            map_zero_to_none(swap_destination_amount.checked_sub(new_destination_amount)?)?;

        // actually add the whole amount coming in
        let new_source_amount = swap_source_amount.checked_add(source_amount)?;
        Some(SwapResult {
            new_source_amount,
            new_destination_amount,
            amount_swapped,
            trade_fee,
            owner_fee,
        })
    }

    /// Calculate the withdraw fee in pool tokens
    fn owner_withdraw_fee(&self, pool_tokens: u128) -> Option<u128> {
        calculate_fee(
            pool_tokens,
            u128::try_from(self.owner_withdraw_fee_numerator).ok()?,
            u128::try_from(self.owner_withdraw_fee_denominator).ok()?,
        )
    }

    /// Calculate the trading fee in trading tokens
    fn trading_fee(&self, trading_tokens: u128) -> Option<u128> {
        calculate_fee(
            trading_tokens,
            u128::try_from(self.trade_fee_numerator).ok()?,
            u128::try_from(self.trade_fee_denominator).ok()?,
        )
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
    const LEN: usize = 48;
    fn unpack_from_slice(input: &[u8]) -> Result<ConstantProductCurve, ProgramError> {
        let input = array_ref![input, 0, 48];
        #[allow(clippy::ptr_offset_with_cast)]
        let (
            trade_fee_numerator,
            trade_fee_denominator,
            owner_trade_fee_numerator,
            owner_trade_fee_denominator,
            owner_withdraw_fee_numerator,
            owner_withdraw_fee_denominator,
        ) = array_refs![input, 8, 8, 8, 8, 8, 8];
        Ok(Self {
            trade_fee_numerator: u64::from_le_bytes(*trade_fee_numerator),
            trade_fee_denominator: u64::from_le_bytes(*trade_fee_denominator),
            owner_trade_fee_numerator: u64::from_le_bytes(*owner_trade_fee_numerator),
            owner_trade_fee_denominator: u64::from_le_bytes(*owner_trade_fee_denominator),
            owner_withdraw_fee_numerator: u64::from_le_bytes(*owner_withdraw_fee_numerator),
            owner_withdraw_fee_denominator: u64::from_le_bytes(*owner_withdraw_fee_denominator),
        })
    }

    fn pack_into_slice(&self, output: &mut [u8]) {
        (self as &dyn DynPack).pack_into_slice(output);
    }
}

impl DynPack for ConstantProductCurve {
    fn pack_into_slice(&self, output: &mut [u8]) {
        let output = array_mut_ref![output, 0, 48];
        let (
            trade_fee_numerator,
            trade_fee_denominator,
            owner_trade_fee_numerator,
            owner_trade_fee_denominator,
            owner_withdraw_fee_numerator,
            owner_withdraw_fee_denominator,
        ) = mut_array_refs![output, 8, 8, 8, 8, 8, 8];
        *trade_fee_numerator = self.trade_fee_numerator.to_le_bytes();
        *trade_fee_denominator = self.trade_fee_denominator.to_le_bytes();
        *owner_trade_fee_numerator = self.owner_trade_fee_numerator.to_le_bytes();
        *owner_trade_fee_denominator = self.owner_trade_fee_denominator.to_le_bytes();
        *owner_withdraw_fee_numerator = self.owner_withdraw_fee_numerator.to_le_bytes();
        *owner_withdraw_fee_denominator = self.owner_withdraw_fee_denominator.to_le_bytes();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_pool_amount() {
        let trade_fee_numerator = 0;
        let trade_fee_denominator = 1;
        let owner_trade_fee_numerator = 0;
        let owner_trade_fee_denominator = 1;
        let owner_withdraw_fee_numerator = 0;
        let owner_withdraw_fee_denominator = 1;
        let calculator = ConstantProductCurve {
            trade_fee_numerator,
            trade_fee_denominator,
            owner_trade_fee_numerator,
            owner_trade_fee_denominator,
            owner_withdraw_fee_numerator,
            owner_withdraw_fee_denominator,
        };
        assert_eq!(calculator.new_pool_supply(), INITIAL_SWAP_POOL_AMOUNT);
    }

    fn check_pool_token_rate(token_a: u128, deposit: u128, supply: u128, expected: Option<u128>) {
        let trade_fee_numerator = 0;
        let trade_fee_denominator = 1;
        let owner_trade_fee_numerator = 0;
        let owner_trade_fee_denominator = 1;
        let owner_withdraw_fee_numerator = 0;
        let owner_withdraw_fee_denominator = 1;
        let calculator = ConstantProductCurve {
            trade_fee_numerator,
            trade_fee_denominator,
            owner_trade_fee_numerator,
            owner_trade_fee_denominator,
            owner_withdraw_fee_numerator,
            owner_withdraw_fee_denominator,
        };
        assert_eq!(
            calculator.pool_tokens_to_trading_tokens(deposit, supply, token_a),
            expected
        );
    }

    #[test]
    fn trading_token_conversion() {
        check_pool_token_rate(2, 5, 10, Some(1));
        check_pool_token_rate(10, 5, 10, Some(5));
        check_pool_token_rate(5, 5, 10, Some(2));
        check_pool_token_rate(5, 5, 10, Some(2));
        check_pool_token_rate(u128::MAX, 5, 10, None);
    }

    #[test]
    fn constant_product_swap_calculation_trade_fee() {
        // calculation on https://github.com/solana-labs/solana-program-library/issues/341
        let swap_source_amount = 1000;
        let swap_destination_amount = 50000;
        let trade_fee_numerator = 1;
        let trade_fee_denominator = 100;
        let owner_trade_fee_numerator = 0;
        let owner_trade_fee_denominator = 0;
        let owner_withdraw_fee_numerator = 0;
        let owner_withdraw_fee_denominator = 0;
        let source_amount = 100;
        let curve = ConstantProductCurve {
            trade_fee_numerator,
            trade_fee_denominator,
            owner_trade_fee_numerator,
            owner_trade_fee_denominator,
            owner_withdraw_fee_numerator,
            owner_withdraw_fee_denominator,
        };
        let result = curve
            .swap(source_amount, swap_source_amount, swap_destination_amount)
            .unwrap();
        assert_eq!(result.new_source_amount, 1100);
        assert_eq!(result.amount_swapped, 4505);
        assert_eq!(result.new_destination_amount, 45495);
        assert_eq!(result.trade_fee, 1);
        assert_eq!(result.owner_fee, 0);
    }

    #[test]
    fn constant_product_swap_calculation_owner_fee() {
        // calculation on https://github.com/solana-labs/solana-program-library/issues/341
        let swap_source_amount = 1000;
        let swap_destination_amount = 50000;
        let trade_fee_numerator = 0;
        let trade_fee_denominator = 0;
        let owner_trade_fee_numerator = 1;
        let owner_trade_fee_denominator = 100;
        let owner_withdraw_fee_numerator = 0;
        let owner_withdraw_fee_denominator = 0;
        let source_amount: u128 = 100;
        let curve = ConstantProductCurve {
            trade_fee_numerator,
            trade_fee_denominator,
            owner_trade_fee_numerator,
            owner_trade_fee_denominator,
            owner_withdraw_fee_numerator,
            owner_withdraw_fee_denominator,
        };
        let result = curve
            .swap(source_amount, swap_source_amount, swap_destination_amount)
            .unwrap();
        assert_eq!(result.new_source_amount, 1100);
        assert_eq!(result.amount_swapped, 4505);
        assert_eq!(result.new_destination_amount, 45495);
        assert_eq!(result.trade_fee, 0);
        assert_eq!(result.owner_fee, 1);
    }

    #[test]
    fn constant_product_swap_no_fee() {
        let swap_source_amount: u128 = 1000;
        let swap_destination_amount: u128 = 50000;
        let source_amount: u128 = 100;
        let curve = ConstantProductCurve::default();
        let result = curve
            .swap(source_amount, swap_source_amount, swap_destination_amount)
            .unwrap();
        assert_eq!(result.new_source_amount, 1100);
        assert_eq!(result.amount_swapped, 4546);
        assert_eq!(result.new_destination_amount, 45454);
    }

    #[test]
    fn flat_swap_calculation() {
        let swap_source_amount = 1000;
        let swap_destination_amount = 50000;
        let trade_fee_numerator = 1;
        let trade_fee_denominator = 100;
        let owner_trade_fee_numerator = 2;
        let owner_trade_fee_denominator = 100;
        let owner_withdraw_fee_numerator = 2;
        let owner_withdraw_fee_denominator = 100;
        let source_amount: u128 = 100;
        let curve = FlatCurve {
            trade_fee_numerator,
            trade_fee_denominator,
            owner_trade_fee_numerator,
            owner_trade_fee_denominator,
            owner_withdraw_fee_numerator,
            owner_withdraw_fee_denominator,
        };
        let result = curve
            .swap(source_amount, swap_source_amount, swap_destination_amount)
            .unwrap();
        let amount_swapped = 97;
        assert_eq!(result.new_source_amount, 1100);
        assert_eq!(result.amount_swapped, amount_swapped);
        assert_eq!(result.trade_fee, 1);
        assert_eq!(result.owner_fee, 2);
        assert_eq!(
            result.new_destination_amount,
            swap_destination_amount - amount_swapped
        );
    }

    #[test]
    fn pack_flat_curve() {
        let trade_fee_numerator = 1;
        let trade_fee_denominator = 4;
        let owner_trade_fee_numerator = 2;
        let owner_trade_fee_denominator = 5;
        let owner_withdraw_fee_numerator = 4;
        let owner_withdraw_fee_denominator = 10;
        let curve = FlatCurve {
            trade_fee_numerator,
            trade_fee_denominator,
            owner_trade_fee_numerator,
            owner_trade_fee_denominator,
            owner_withdraw_fee_numerator,
            owner_withdraw_fee_denominator,
        };

        let mut packed = [0u8; FlatCurve::LEN];
        Pack::pack_into_slice(&curve, &mut packed[..]);
        let unpacked = FlatCurve::unpack(&packed).unwrap();
        assert_eq!(curve, unpacked);

        let mut packed = vec![];
        packed.extend_from_slice(&trade_fee_numerator.to_le_bytes());
        packed.extend_from_slice(&trade_fee_denominator.to_le_bytes());
        packed.extend_from_slice(&owner_trade_fee_numerator.to_le_bytes());
        packed.extend_from_slice(&owner_trade_fee_denominator.to_le_bytes());
        packed.extend_from_slice(&owner_withdraw_fee_numerator.to_le_bytes());
        packed.extend_from_slice(&owner_withdraw_fee_denominator.to_le_bytes());
        let unpacked = FlatCurve::unpack(&packed).unwrap();
        assert_eq!(curve, unpacked);
    }

    #[test]
    fn pack_constant_product_curve() {
        let trade_fee_numerator = 1;
        let trade_fee_denominator = 4;
        let owner_trade_fee_numerator = 2;
        let owner_trade_fee_denominator = 5;
        let owner_withdraw_fee_numerator = 4;
        let owner_withdraw_fee_denominator = 10;
        let curve = ConstantProductCurve {
            trade_fee_numerator,
            trade_fee_denominator,
            owner_trade_fee_numerator,
            owner_trade_fee_denominator,
            owner_withdraw_fee_numerator,
            owner_withdraw_fee_denominator,
        };

        let mut packed = [0u8; ConstantProductCurve::LEN];
        Pack::pack_into_slice(&curve, &mut packed[..]);
        let unpacked = ConstantProductCurve::unpack(&packed).unwrap();
        assert_eq!(curve, unpacked);

        let mut packed = vec![];
        packed.extend_from_slice(&trade_fee_numerator.to_le_bytes());
        packed.extend_from_slice(&trade_fee_denominator.to_le_bytes());
        packed.extend_from_slice(&owner_trade_fee_numerator.to_le_bytes());
        packed.extend_from_slice(&owner_trade_fee_denominator.to_le_bytes());
        packed.extend_from_slice(&owner_withdraw_fee_numerator.to_le_bytes());
        packed.extend_from_slice(&owner_withdraw_fee_denominator.to_le_bytes());
        let unpacked = ConstantProductCurve::unpack(&packed).unwrap();
        assert_eq!(curve, unpacked);
    }

    #[test]
    fn pack_swap_curve() {
        let trade_fee_numerator = 1;
        let trade_fee_denominator = 4;
        let owner_trade_fee_numerator = 2;
        let owner_trade_fee_denominator = 5;
        let owner_withdraw_fee_numerator = 4;
        let owner_withdraw_fee_denominator = 10;
        let curve = ConstantProductCurve {
            trade_fee_numerator,
            trade_fee_denominator,
            owner_trade_fee_numerator,
            owner_trade_fee_denominator,
            owner_withdraw_fee_numerator,
            owner_withdraw_fee_denominator,
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
        packed.extend_from_slice(&[0u8; 16]); // padding
        let unpacked = SwapCurve::unpack_from_slice(&packed).unwrap();
        assert_eq!(swap_curve, unpacked);
    }
}
