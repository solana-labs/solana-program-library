use std::convert::TryInto;

use super::*;
use crate::{
    error::LendingError,
    math::{Decimal, Rate, TryDiv, TryMul, TrySub},
};
use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use solana_program::{
    clock::Slot,
    entrypoint::ProgramResult,
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};

/// Obligation liquidity state
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ObligationLiquidity {
    /// Version of the obligation liquidity
    pub version: u8,
    /// Reserve which liquidity tokens were borrowed from
    pub borrow_reserve: Pubkey,
    /// Borrow rate used for calculating interest
    pub cumulative_borrow_rate_wads: Decimal,
    /// Amount of liquidity tokens borrowed for an obligation plus interest
    pub borrowed_wads: Decimal,
    /// Market value of liquidity
    pub market_value: u64,
    /// Last slot when market value and accrued interest updated
    pub last_update_slot: Slot,
}

/// Create new obligation liquidity
pub struct NewObligationLiquidityParams {
    /// Borrow reserve address
    pub borrow_reserve: Pubkey,
    /// Current slot
    pub current_slot: Slot,
}

impl ObligationLiquidity {
    /// Create new obligation liquidity
    pub fn new(params: NewObligationLiquidityParams) -> Self {
        let NewObligationLiquidityParams {
            borrow_reserve,
            current_slot,
        } = params;

        Self {
            version: PROGRAM_VERSION,
            borrow_reserve,
            cumulative_borrow_rate_wads: Decimal::one(),
            borrowed_wads: Decimal::zero(),
            market_value: 0,
            last_update_slot: current_slot,
        }
    }

    /// Maximum amount of loan that can be closed out by a liquidator due to the remaining balance
    /// being too small to be liquidated normally.
    pub fn max_closeable_amount(&self) -> Result<u64, ProgramError> {
        if self.borrowed_wads < Decimal::from(CLOSEABLE_AMOUNT) {
            self.borrowed_wads.try_ceil_u64()
        } else {
            Ok(0)
        }
    }

    /// Maximum amount of loan that can be repaid by liquidators
    pub fn max_liquidation_amount(&self) -> Result<u64, ProgramError> {
        Ok(self
            .borrowed_wads
            .try_mul(Rate::from_percent(LIQUIDATION_CLOSE_FACTOR))?
            .try_floor_u64()?)
    }

    /// Accrue interest
    pub fn accrue_interest(&mut self, cumulative_borrow_rate_wads: Decimal) -> ProgramResult {
        if cumulative_borrow_rate_wads < self.cumulative_borrow_rate_wads {
            return Err(LendingError::NegativeInterestRate.into());
        }

        let compounded_interest_rate: Rate = cumulative_borrow_rate_wads
            .try_div(self.cumulative_borrow_rate_wads)?
            .try_into()?;

        self.borrowed_wads = self.borrowed_wads.try_mul(compounded_interest_rate)?;

        self.cumulative_borrow_rate_wads = cumulative_borrow_rate_wads;

        Ok(())
    }

    /// Return slots elapsed since last update
    fn update_slot(&mut self, slot: Slot) -> u64 {
        // @TODO: checked math?
        let slots_elapsed = slot - self.last_update_slot;
        self.last_update_slot = slot;
        slots_elapsed
    }
}

impl Sealed for ObligationLiquidity {}
impl IsInitialized for ObligationLiquidity {
    fn is_initialized(&self) -> bool {
        self.version != UNINITIALIZED_VERSION
    }
}

const OBLIGATION_LIQUIDITY_LEN: usize = 209; // 1 + 32 + 16 + 16 + 8 + 8 + 128
impl Pack for ObligationLiquidity {
    const LEN: usize = OBLIGATION_LIQUIDITY_LEN;

    fn pack_into_slice(&self, output: &mut [u8]) {
        let output = array_mut_ref![output, 0, OBLIGATION_LIQUIDITY_LEN];
        let (
            version,
            borrow_reserve,
            cumulative_borrow_rate_wads,
            borrowed_wads,
            market_value,
            last_update_slot,
            _padding,
        ) = mut_array_refs![output, 1, PUBKEY_LEN, 16, 16, 8, 8, 128];

        *version = self.version.to_le_bytes();
        borrow_reserve.copy_from_slice(self.borrow_reserve.as_ref());
        pack_decimal(
            self.cumulative_borrow_rate_wads,
            cumulative_borrow_rate_wads,
        );
        pack_decimal(self.borrowed_wads, borrowed_wads);
        *market_value = self.market_value.to_le_bytes();
        *last_update_slot = self.last_update_slot.to_le_bytes();
    }

    /// Unpacks a byte buffer into a [ObligationInfo](struct.ObligationInfo.html).
    fn unpack_from_slice(input: &[u8]) -> Result<Self, ProgramError> {
        let input = array_ref![input, 0, OBLIGATION_LIQUIDITY_LEN];
        #[allow(clippy::ptr_offset_with_cast)]
        let (
            version,
            borrow_reserve,
            cumulative_borrow_rate_wads,
            borrowed_wads,
            market_value,
            last_update_slot,
            _padding,
        ) = array_refs![input, 1, PUBKEY_LEN, 16, 16, 8, 8, 128];

        Ok(Self {
            version: u8::from_le_bytes(*version),
            borrow_reserve: Pubkey::new_from_array(*borrow_reserve),
            cumulative_borrow_rate_wads: unpack_decimal(cumulative_borrow_rate_wads),
            borrowed_wads: unpack_decimal(borrowed_wads),
            market_value: u64::from_le_bytes(*market_value),
            last_update_slot: u64::from_le_bytes(*last_update_slot),
        })
    }
}
