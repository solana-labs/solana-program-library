use super::*;
use crate::{
    error::LendingError,
    math::{Decimal, Rate, TryAdd, TryDiv, TryMul, TrySub},
};
use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use solana_program::{
    clock::Slot,
    entrypoint::ProgramResult,
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};
use std::convert::TryInto;

/// Obligation liquidity state
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ObligationLiquidity {
    /// Version of the obligation liquidity
    pub version: u8,
    /// Last slot when market value and accrued interest updated; set to 0 if borrowed wads changed
    pub last_update_slot: Slot,
    /// Obligation the liquidity is associated with
    pub obligation: Pubkey,
    /// Reserve which liquidity tokens were borrowed from
    pub borrow_reserve: Pubkey,
    /// Borrow rate used for calculating interest
    pub cumulative_borrow_rate_wads: Decimal,
    /// Amount of liquidity tokens borrowed for an obligation plus interest
    pub borrowed_wads: Decimal,
    /// Market value of liquidity
    pub market_value: Decimal,
}

/// Create new obligation liquidity
pub struct NewObligationLiquidityParams {
    /// Obligation address
    pub obligation: Pubkey,
    /// Borrow reserve address
    pub borrow_reserve: Pubkey,
}

impl ObligationLiquidity {
    /// Create new obligation liquidity
    pub fn new(params: NewObligationLiquidityParams) -> Self {
        let NewObligationLiquidityParams {
            obligation,
            borrow_reserve,
        } = params;

        Self {
            version: PROGRAM_VERSION,
            last_update_slot: 0,
            obligation,
            borrow_reserve,
            cumulative_borrow_rate_wads: Decimal::one(),
            borrowed_wads: Decimal::zero(),
            market_value: Decimal::zero(),
        }
    }

    /// Decrease borrowed liquidity
    pub fn repay(&mut self, liquidity_amount: u64) -> ProgramResult {
        self.borrowed_wads = self.borrowed_wads.try_sub(liquidity_amount.into())?;
        Ok(())
    }

    /// Increase borrowed liquidity
    pub fn borrow(&mut self, liquidity_amount: u64) -> ProgramResult {
        self.borrowed_wads = self.borrowed_wads.try_add(liquidity_amount.into())?;
        Ok(())
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
        self.borrowed_wads
            .try_mul(Rate::from_percent(LIQUIDATION_CLOSE_FACTOR))?
            .try_floor_u64()
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

    /// Update market value of liquidity
    pub fn update_market_value(
        &mut self,
        converter: impl TokenConverter,
        from_token_mint: &Pubkey,
    ) -> ProgramResult {
        // @TODO: this may be slow/inaccurate for large amounts depending on dex market
        self.market_value = converter.convert(self.borrowed_wads, from_token_mint)?;
        Ok(())
    }

    /// Return slots elapsed since given slot
    pub fn slots_elapsed(&self, slot: Slot) -> Result<u64, ProgramError> {
        let slots_elapsed = slot
            .checked_sub(self.last_update_slot)
            .ok_or(LendingError::MathOverflow)?;
        Ok(slots_elapsed)
    }

    /// Set last update slot
    pub fn update_slot(&mut self, slot: Slot) {
        self.last_update_slot = slot;
    }

    /// Set last update slot to 0
    pub fn mark_stale(&mut self) {
        self.update_slot(0);
    }

    /// Check if last update slot is too long ago
    pub fn is_stale(&self, slot: Slot) -> Result<bool, ProgramError> {
        Ok(self.last_update_slot == 0 || self.slots_elapsed(slot)? > STALE_AFTER_SLOTS)
    }
}

impl Sealed for ObligationLiquidity {}
impl IsInitialized for ObligationLiquidity {
    fn is_initialized(&self) -> bool {
        self.version != UNINITIALIZED_VERSION
    }
}

const OBLIGATION_LIQUIDITY_LEN: usize = 249; // 1 + 8 + 32 + 32 + 16 + 16 + 16 + 128
impl Pack for ObligationLiquidity {
    const LEN: usize = OBLIGATION_LIQUIDITY_LEN;

    fn pack_into_slice(&self, output: &mut [u8]) {
        let output = array_mut_ref![output, 0, OBLIGATION_LIQUIDITY_LEN];
        let (
            version,
            last_update_slot,
            obligation,
            borrow_reserve,
            cumulative_borrow_rate_wads,
            borrowed_wads,
            market_value,
            _padding,
        ) = mut_array_refs![output, 1, 8, PUBKEY_LEN, PUBKEY_LEN, 16, 16, 16, 128];

        *version = self.version.to_le_bytes();
        *last_update_slot = self.last_update_slot.to_le_bytes();
        obligation.copy_from_slice(self.obligation.as_ref());
        borrow_reserve.copy_from_slice(self.borrow_reserve.as_ref());
        pack_decimal(
            self.cumulative_borrow_rate_wads,
            cumulative_borrow_rate_wads,
        );
        pack_decimal(self.borrowed_wads, borrowed_wads);
        pack_decimal(self.market_value, market_value);
    }

    /// Unpacks a byte buffer into a [ObligationInfo](struct.ObligationInfo.html).
    fn unpack_from_slice(input: &[u8]) -> Result<Self, ProgramError> {
        let input = array_ref![input, 0, OBLIGATION_LIQUIDITY_LEN];
        #[allow(clippy::ptr_offset_with_cast)]
        let (
            version,
            last_update_slot,
            obligation,
            borrow_reserve,
            cumulative_borrow_rate_wads,
            borrowed_wads,
            market_value,
            _padding,
        ) = array_refs![input, 1, 8, PUBKEY_LEN, PUBKEY_LEN, 16, 16, 16, 128];

        Ok(Self {
            version: u8::from_le_bytes(*version),
            last_update_slot: u64::from_le_bytes(*last_update_slot),
            obligation: Pubkey::new_from_array(*obligation),
            borrow_reserve: Pubkey::new_from_array(*borrow_reserve),
            cumulative_borrow_rate_wads: unpack_decimal(cumulative_borrow_rate_wads),
            borrowed_wads: unpack_decimal(borrowed_wads),
            market_value: unpack_decimal(market_value),
        })
    }
}
