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
    /// Last update to accrued interest, borrowed wads, or their market value
    pub last_update: LastUpdate,
    /// Obligation the liquidity is associated with
    pub obligation: Pubkey,
    /// Reserve which liquidity tokens were borrowed from
    pub borrow_reserve: Pubkey,
    /// Borrow rate used for calculating interest
    pub cumulative_borrow_rate_wads: Decimal,
    /// Amount of liquidity borrowed plus interest
    pub borrowed_amount_wads: Decimal,
    /// Market value of liquidity in quote currency
    pub value: Decimal,
}

/// Create new obligation liquidity
pub struct NewObligationLiquidityParams {
    /// Current slot
    pub current_slot: Slot,
    /// Obligation address
    pub obligation: Pubkey,
    /// Borrow reserve address
    pub borrow_reserve: Pubkey,
}

impl ObligationLiquidity {
    /// Create new obligation liquidity
    pub fn new(params: NewObligationLiquidityParams) -> Self {
        let NewObligationLiquidityParams {
            current_slot,
            obligation,
            borrow_reserve,
        } = params;

        Self {
            version: PROGRAM_VERSION,
            last_update: LastUpdate::new(current_slot),
            obligation,
            borrow_reserve,
            cumulative_borrow_rate_wads: Decimal::one(),
            borrowed_amount_wads: Decimal::zero(),
            value: Decimal::zero(),
        }
    }

    /// Decrease borrowed liquidity
    pub fn repay(&mut self, settle_amount: Decimal) -> ProgramResult {
        self.borrowed_amount_wads = self.borrowed_amount_wads.try_sub(settle_amount)?;
        Ok(())
    }

    /// Increase borrowed liquidity
    pub fn borrow(&mut self, borrow_amount: u64) -> ProgramResult {
        self.borrowed_amount_wads = self.borrowed_amount_wads.try_add(borrow_amount.into())?;
        Ok(())
    }

    /// Accrue interest
    pub fn accrue_interest(&mut self, cumulative_borrow_rate_wads: Decimal) -> ProgramResult {
        if cumulative_borrow_rate_wads < self.cumulative_borrow_rate_wads {
            return Err(LendingError::NegativeInterestRate.into());
        }

        let compounded_interest_rate: Rate = cumulative_borrow_rate_wads
            .try_div(self.cumulative_borrow_rate_wads)?
            .try_into()?;

        self.borrowed_amount_wads = self
            .borrowed_amount_wads
            .try_mul(compounded_interest_rate)?;
        self.cumulative_borrow_rate_wads = cumulative_borrow_rate_wads;

        Ok(())
    }

    /// Update market value of liquidity
    pub fn update_value(
        &mut self,
        token_converter: impl TokenConverter,
        from_token_mint: &Pubkey,
    ) -> ProgramResult {
        // @TODO: this may be slow/inaccurate for large amounts depending on dex market
        self.value = token_converter.convert(self.borrowed_amount_wads, from_token_mint)?;
        Ok(())
    }
}

impl Sealed for ObligationLiquidity {}
impl IsInitialized for ObligationLiquidity {
    fn is_initialized(&self) -> bool {
        self.version != UNINITIALIZED_VERSION
    }
}

const OBLIGATION_LIQUIDITY_LEN: usize = 250; // 1 + 8 + 1 + 32 + 32 + 16 + 16 + 16 + 128
impl Pack for ObligationLiquidity {
    const LEN: usize = OBLIGATION_LIQUIDITY_LEN;

    fn pack_into_slice(&self, output: &mut [u8]) {
        let output = array_mut_ref![output, 0, OBLIGATION_LIQUIDITY_LEN];
        let (
            version,
            last_update_slot,
            last_update_stale,
            obligation,
            borrow_reserve,
            cumulative_borrow_rate_wads,
            borrowed_wads,
            value,
            _padding,
        ) = mut_array_refs![output, 1, 8, 1, PUBKEY_LEN, PUBKEY_LEN, 16, 16, 16, 128];

        *version = self.version.to_le_bytes();
        *last_update_slot = self.last_update.slot.to_le_bytes();
        *last_update_stale = u8::from(self.last_update.stale).to_le_bytes();
        obligation.copy_from_slice(self.obligation.as_ref());
        borrow_reserve.copy_from_slice(self.borrow_reserve.as_ref());
        pack_decimal(
            self.cumulative_borrow_rate_wads,
            cumulative_borrow_rate_wads,
        );
        pack_decimal(self.borrowed_amount_wads, borrowed_wads);
        pack_decimal(self.value, value);
    }

    /// Unpacks a byte buffer into a [ObligationInfo](struct.ObligationInfo.html).
    fn unpack_from_slice(input: &[u8]) -> Result<Self, ProgramError> {
        let input = array_ref![input, 0, OBLIGATION_LIQUIDITY_LEN];
        #[allow(clippy::ptr_offset_with_cast)]
        let (
            version,
            last_update_slot,
            last_update_stale,
            obligation,
            borrow_reserve,
            cumulative_borrow_rate_wads,
            borrowed_wads,
            value,
            _padding,
        ) = array_refs![input, 1, 8, 1, PUBKEY_LEN, PUBKEY_LEN, 16, 16, 16, 128];

        Ok(Self {
            version: u8::from_le_bytes(*version),
            last_update: LastUpdate {
                slot: u64::from_le_bytes(*last_update_slot),
                stale: bool::from(u8::from_le_bytes(*last_update_stale)),
            },
            obligation: Pubkey::new_from_array(*obligation),
            borrow_reserve: Pubkey::new_from_array(*borrow_reserve),
            cumulative_borrow_rate_wads: unpack_decimal(cumulative_borrow_rate_wads),
            borrowed_amount_wads: unpack_decimal(borrowed_wads),
            value: unpack_decimal(value),
        })
    }
}
