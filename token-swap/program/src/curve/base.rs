//! Base curve implementation

#[cfg(feature = "fuzz")]
use arbitrary::Arbitrary;
use {
    crate::curve::{
        calculator::{CurveCalculator, RoundDirection, SwapWithoutFeesResult, TradeDirection},
        constant_price::ConstantPriceCurve,
        constant_product::ConstantProductCurve,
        fees::Fees,
        offset::OffsetCurve,
    },
    arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs},
    solana_program::{
        program_error::ProgramError,
        program_pack::{Pack, Sealed},
    },
    std::{
        convert::{TryFrom, TryInto},
        fmt::Debug,
        sync::Arc,
    },
};

/// Curve types supported by the token-swap program.
#[cfg_attr(feature = "fuzz", derive(Arbitrary))]
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CurveType {
    /// Uniswap-style constant product curve, invariant = token_a_amount *
    /// token_b_amount
    ConstantProduct,
    /// Flat line, always providing 1:1 from one token to another
    ConstantPrice,
    /// Offset curve, like Uniswap, but the token B side has a faked offset
    Offset,
}

/// Encodes all results of swapping from a source token to a destination token
#[derive(Debug, PartialEq)]
pub struct SwapResult {
    /// New amount of source token
    pub new_swap_source_amount: u128,
    /// New amount of destination token
    pub new_swap_destination_amount: u128,
    /// Amount of source token swapped (includes fees)
    pub source_amount_swapped: u128,
    /// Amount of destination token swapped
    pub destination_amount_swapped: u128,
    /// Amount of source tokens going to pool holders
    pub trade_fee: u128,
    /// Amount of source tokens going to owner
    pub owner_fee: u128,
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
    pub calculator: Arc<dyn CurveCalculator + Sync + Send>,
}

impl SwapCurve {
    /// Subtract fees and calculate how much destination token will be provided
    /// given an amount of source token.
    pub fn swap(
        &self,
        source_amount: u128,
        swap_source_amount: u128,
        swap_destination_amount: u128,
        trade_direction: TradeDirection,
        fees: &Fees,
    ) -> Option<SwapResult> {
        // debit the fee to calculate the amount swapped
        let trade_fee = fees.trading_fee(source_amount)?;
        let owner_fee = fees.owner_trading_fee(source_amount)?;

        let total_fees = trade_fee.checked_add(owner_fee)?;
        let source_amount_less_fees = source_amount.checked_sub(total_fees)?;

        let SwapWithoutFeesResult {
            source_amount_swapped,
            destination_amount_swapped,
        } = self.calculator.swap_without_fees(
            source_amount_less_fees,
            swap_source_amount,
            swap_destination_amount,
            trade_direction,
        )?;

        let source_amount_swapped = source_amount_swapped.checked_add(total_fees)?;
        Some(SwapResult {
            new_swap_source_amount: swap_source_amount.checked_add(source_amount_swapped)?,
            new_swap_destination_amount: swap_destination_amount
                .checked_sub(destination_amount_swapped)?,
            source_amount_swapped,
            destination_amount_swapped,
            trade_fee,
            owner_fee,
        })
    }

    /// Get the amount of pool tokens for the deposited amount of token A or B
    pub fn deposit_single_token_type(
        &self,
        source_amount: u128,
        swap_token_a_amount: u128,
        swap_token_b_amount: u128,
        pool_supply: u128,
        trade_direction: TradeDirection,
        fees: &Fees,
    ) -> Option<u128> {
        if source_amount == 0 {
            return Some(0);
        }
        // Get the trading fee incurred if *half* the source amount is swapped
        // for the other side. Reference at:
        // https://github.com/balancer-labs/balancer-core/blob/f4ed5d65362a8d6cec21662fb6eae233b0babc1f/contracts/BMath.sol#L117
        let half_source_amount = std::cmp::max(1, source_amount.checked_div(2)?);
        let trade_fee = fees.trading_fee(half_source_amount)?;
        let owner_fee = fees.owner_trading_fee(half_source_amount)?;
        let total_fees = trade_fee.checked_add(owner_fee)?;
        let source_amount = source_amount.checked_sub(total_fees)?;
        self.calculator.deposit_single_token_type(
            source_amount,
            swap_token_a_amount,
            swap_token_b_amount,
            pool_supply,
            trade_direction,
        )
    }

    /// Get the amount of pool tokens for the withdrawn amount of token A or B
    pub fn withdraw_single_token_type_exact_out(
        &self,
        source_amount: u128,
        swap_token_a_amount: u128,
        swap_token_b_amount: u128,
        pool_supply: u128,
        trade_direction: TradeDirection,
        fees: &Fees,
    ) -> Option<u128> {
        if source_amount == 0 {
            return Some(0);
        }
        // Since we want to get the amount required to get the exact amount out,
        // we need the inverse trading fee incurred if *half* the source amount
        // is swapped for the other side. Reference at:
        // https://github.com/balancer-labs/balancer-core/blob/f4ed5d65362a8d6cec21662fb6eae233b0babc1f/contracts/BMath.sol#L117
        let half_source_amount = source_amount.checked_add(1)?.checked_div(2)?; // round up
        let pre_fee_source_amount = fees.pre_trading_fee_amount(half_source_amount)?;
        let source_amount = source_amount
            .checked_sub(half_source_amount)?
            .checked_add(pre_fee_source_amount)?;
        self.calculator.withdraw_single_token_type_exact_out(
            source_amount,
            swap_token_a_amount,
            swap_token_b_amount,
            pool_supply,
            trade_direction,
            RoundDirection::Ceiling,
        )
    }
}

/// Default implementation for SwapCurve cannot be derived because of
/// the contained Arc.
impl Default for SwapCurve {
    fn default() -> Self {
        let curve_type: CurveType = Default::default();
        let calculator: ConstantProductCurve = Default::default();
        Self {
            curve_type,
            calculator: Arc::new(calculator),
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
    /// Size of encoding of all curve parameters, which include fees and any
    /// other constants used to calculate swaps, deposits, and withdrawals.
    /// This includes 1 byte for the type, and 72 for the calculator to use as
    /// it needs.  Some calculators may be smaller than 72 bytes.
    const LEN: usize = 33;

    /// Unpacks a byte buffer into a SwapCurve
    fn unpack_from_slice(input: &[u8]) -> Result<Self, ProgramError> {
        let input = array_ref![input, 0, 33];
        #[allow(clippy::ptr_offset_with_cast)]
        let (curve_type, calculator) = array_refs![input, 1, 32];
        let curve_type = curve_type[0].try_into()?;
        Ok(Self {
            curve_type,
            calculator: match curve_type {
                CurveType::ConstantProduct => {
                    Arc::new(ConstantProductCurve::unpack_from_slice(calculator)?)
                }
                CurveType::ConstantPrice => {
                    Arc::new(ConstantPriceCurve::unpack_from_slice(calculator)?)
                }
                CurveType::Offset => Arc::new(OffsetCurve::unpack_from_slice(calculator)?),
            },
        })
    }

    /// Pack SwapCurve into a byte buffer
    fn pack_into_slice(&self, output: &mut [u8]) {
        let output = array_mut_ref![output, 0, 33];
        let (curve_type, calculator) = mut_array_refs![output, 1, 32];
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
            1 => Ok(CurveType::ConstantPrice),
            2 => Ok(CurveType::Offset),
            _ => Err(ProgramError::InvalidAccountData),
        }
    }
}

#[cfg(test)]
mod test {
    use {super::*, crate::curve::calculator::test::total_and_intermediate, proptest::prelude::*};

    #[test]
    fn pack_swap_curve() {
        let curve = ConstantProductCurve {};
        let curve_type = CurveType::ConstantProduct;
        let swap_curve = SwapCurve {
            curve_type,
            calculator: Arc::new(curve),
        };

        let mut packed = [0u8; SwapCurve::LEN];
        Pack::pack_into_slice(&swap_curve, &mut packed[..]);
        let unpacked = SwapCurve::unpack_from_slice(&packed).unwrap();
        assert_eq!(swap_curve, unpacked);

        let mut packed = vec![curve_type as u8];
        packed.extend_from_slice(&[0u8; 32]); // 32 bytes reserved for curve
        let unpacked = SwapCurve::unpack_from_slice(&packed).unwrap();
        assert_eq!(swap_curve, unpacked);
    }

    #[test]
    fn constant_product_trade_fee() {
        // calculation on https://github.com/solana-labs/solana-program-library/issues/341
        let swap_source_amount = 1000;
        let swap_destination_amount = 50000;
        let trade_fee_numerator = 1;
        let trade_fee_denominator = 100;
        let owner_trade_fee_numerator = 0;
        let owner_trade_fee_denominator = 0;
        let owner_withdraw_fee_numerator = 0;
        let owner_withdraw_fee_denominator = 0;
        let host_fee_numerator = 0;
        let host_fee_denominator = 0;

        let fees = Fees {
            trade_fee_numerator,
            trade_fee_denominator,
            owner_trade_fee_numerator,
            owner_trade_fee_denominator,
            owner_withdraw_fee_numerator,
            owner_withdraw_fee_denominator,
            host_fee_numerator,
            host_fee_denominator,
        };
        let source_amount = 100;
        let curve = ConstantProductCurve {};
        let swap_curve = SwapCurve {
            curve_type: CurveType::ConstantProduct,
            calculator: Arc::new(curve),
        };
        let result = swap_curve
            .swap(
                source_amount,
                swap_source_amount,
                swap_destination_amount,
                TradeDirection::AtoB,
                &fees,
            )
            .unwrap();
        assert_eq!(result.new_swap_source_amount, 1100);
        assert_eq!(result.destination_amount_swapped, 4504);
        assert_eq!(result.new_swap_destination_amount, 45496);
        assert_eq!(result.trade_fee, 1);
        assert_eq!(result.owner_fee, 0);
    }

    #[test]
    fn constant_product_owner_fee() {
        // calculation on https://github.com/solana-labs/solana-program-library/issues/341
        let swap_source_amount = 1000;
        let swap_destination_amount = 50000;
        let trade_fee_numerator = 0;
        let trade_fee_denominator = 0;
        let owner_trade_fee_numerator = 1;
        let owner_trade_fee_denominator = 100;
        let owner_withdraw_fee_numerator = 0;
        let owner_withdraw_fee_denominator = 0;
        let host_fee_numerator = 0;
        let host_fee_denominator = 0;
        let fees = Fees {
            trade_fee_numerator,
            trade_fee_denominator,
            owner_trade_fee_numerator,
            owner_trade_fee_denominator,
            owner_withdraw_fee_numerator,
            owner_withdraw_fee_denominator,
            host_fee_numerator,
            host_fee_denominator,
        };
        let source_amount: u128 = 100;
        let curve = ConstantProductCurve {};
        let swap_curve = SwapCurve {
            curve_type: CurveType::ConstantProduct,
            calculator: Arc::new(curve),
        };
        let result = swap_curve
            .swap(
                source_amount,
                swap_source_amount,
                swap_destination_amount,
                TradeDirection::AtoB,
                &fees,
            )
            .unwrap();
        assert_eq!(result.new_swap_source_amount, 1100);
        assert_eq!(result.destination_amount_swapped, 4504);
        assert_eq!(result.new_swap_destination_amount, 45496);
        assert_eq!(result.trade_fee, 0);
        assert_eq!(result.owner_fee, 1);
    }

    #[test]
    fn constant_product_no_fee() {
        let swap_source_amount: u128 = 1_000;
        let swap_destination_amount: u128 = 50_000;
        let source_amount: u128 = 100;
        let curve = ConstantProductCurve;
        let fees = Fees::default();
        let swap_curve = SwapCurve {
            curve_type: CurveType::ConstantProduct,
            calculator: Arc::new(curve),
        };
        let result = swap_curve
            .swap(
                source_amount,
                swap_source_amount,
                swap_destination_amount,
                TradeDirection::AtoB,
                &fees,
            )
            .unwrap();
        assert_eq!(result.new_swap_source_amount, 1100);
        assert_eq!(result.destination_amount_swapped, 4545);
        assert_eq!(result.new_swap_destination_amount, 45455);
    }

    fn one_sided_deposit_vs_swap(
        source_amount: u128,
        swap_source_amount: u128,
        swap_destination_amount: u128,
        pool_supply: u128,
        fees: Fees,
    ) -> (u128, u128) {
        let curve = ConstantProductCurve;
        let swap_curve = SwapCurve {
            curve_type: CurveType::ConstantProduct,
            calculator: Arc::new(curve),
        };
        // do the A to B swap
        let results = swap_curve
            .swap(
                source_amount,
                swap_source_amount,
                swap_destination_amount,
                TradeDirection::AtoB,
                &fees,
            )
            .unwrap();

        // deposit just A, get pool tokens
        let deposit_pool_tokens = swap_curve
            .deposit_single_token_type(
                results.source_amount_swapped,
                swap_source_amount,
                swap_destination_amount,
                pool_supply,
                TradeDirection::AtoB,
                &fees,
            )
            .unwrap();
        let withdraw_pool_tokens = swap_curve
            .withdraw_single_token_type_exact_out(
                results.destination_amount_swapped,
                swap_source_amount + results.source_amount_swapped,
                swap_destination_amount,
                pool_supply + deposit_pool_tokens,
                TradeDirection::BtoA,
                &fees,
            )
            .unwrap();
        (withdraw_pool_tokens, deposit_pool_tokens)
    }

    #[test]
    fn one_sided_equals_swap_with_fee_specific() {
        let pool_supply: u128 = 1_000_000;
        let swap_source_amount: u128 = 1_000_000;
        let swap_destination_amount: u128 = 50_000_000;
        let source_amount: u128 = 10_000;
        let fees = Fees {
            trade_fee_numerator: 25,
            trade_fee_denominator: 1_000,
            owner_trade_fee_numerator: 5,
            owner_trade_fee_denominator: 1_000,
            ..Fees::default()
        };
        let (withdraw_pool_tokens, deposit_pool_tokens) = one_sided_deposit_vs_swap(
            source_amount,
            swap_source_amount,
            swap_destination_amount,
            pool_supply,
            fees,
        );
        // these checks *must* always hold
        assert!(withdraw_pool_tokens >= deposit_pool_tokens);
        let epsilon = 2;
        assert!(withdraw_pool_tokens - deposit_pool_tokens <= epsilon);

        // these checks may change if the calc is updated
        assert_eq!(withdraw_pool_tokens, 4914);
        assert_eq!(deposit_pool_tokens, 4912);
    }

    proptest! {
        #[test]
        fn one_sided_equals_swap_with_fee(
            (swap_source_amount, source_amount) in total_and_intermediate(u64::MAX),
            swap_destination_amount in 1..u64::MAX,
            pool_supply in 1..u64::MAX,
        ) {
            let fees = Fees {
                trade_fee_numerator: 25,
                trade_fee_denominator: 1_000,
                owner_trade_fee_numerator: 5,
                owner_trade_fee_denominator: 1_000,
                ..Fees::default()
            };
            let (withdraw_pool_tokens, deposit_pool_tokens) = one_sided_deposit_vs_swap(
                pool_supply.into(),
                swap_source_amount.into(),
                swap_destination_amount.into(),
                source_amount.into(),
                fees
            );
            // the cost to withdraw B must always be higher than the amount gained through deposit
            assert!(withdraw_pool_tokens >= deposit_pool_tokens);
        }

        #[test]
        fn one_sided_equals_swap_with_withdrawal_fee(
            (swap_source_amount, source_amount) in total_and_intermediate(u64::MAX),
            swap_destination_amount in 1..u64::MAX,
            pool_supply in 1..u64::MAX,
        ) {
            let fees = Fees {
                trade_fee_numerator: 25,
                trade_fee_denominator: 1_000,
                owner_trade_fee_numerator: 5,
                owner_trade_fee_denominator: 1_000,
                owner_withdraw_fee_numerator: 1,
                owner_withdraw_fee_denominator: 1_000,
                ..Fees::default()
            };
            let (withdraw_pool_tokens, deposit_pool_tokens) = one_sided_deposit_vs_swap(
                pool_supply.into(),
                swap_source_amount.into(),
                swap_destination_amount.into(),
                source_amount.into(),
                fees
            );
            // the cost to withdraw B must always be higher than the amount gained through deposit
            assert!(withdraw_pool_tokens >= deposit_pool_tokens);
        }

        #[test]
        fn one_sided_equals_swap_without_fee(
            (swap_source_amount, source_amount) in total_and_intermediate(u64::MAX),
            swap_destination_amount in 1..u64::MAX,
            pool_supply in 1..u64::MAX,
        ) {
            let fees = Fees::default();
            let (withdraw_pool_tokens, deposit_pool_tokens) = one_sided_deposit_vs_swap(
                pool_supply.into(),
                swap_source_amount.into(),
                swap_destination_amount.into(),
                source_amount.into(),
                fees
            );
            let difference = if withdraw_pool_tokens >= deposit_pool_tokens {
                withdraw_pool_tokens - deposit_pool_tokens
            } else {
                deposit_pool_tokens - withdraw_pool_tokens
            };
            // Accurate to one part in 1,000,000 -- without fees, it can go either
            // way due to vast differences in the pool token and trading token
            // amounts.
            // For example, if there's only 1 pool token and 1 destination token,
            // but a source amount of 1,000,000,000, we can lose up to 1,000,000,000
            // in precision during an operation.
            // See the proptests in calculator.rs for more specific versions.
            let epsilon = std::cmp::max(1, withdraw_pool_tokens / 1_000_000);
            assert!(
                difference <= epsilon,
                "difference between {} and {} expected to be less than {}, actually {}",
                withdraw_pool_tokens,
                deposit_pool_tokens,
                epsilon,
                difference
            );
        }
    }
}
