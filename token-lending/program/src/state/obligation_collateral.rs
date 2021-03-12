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

/// Obligation collateral state
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ObligationCollateral {
    /// Version of the obligation collateral
    pub version: u8,
    /// Reserve which collateral tokens were deposited into
    pub deposit_reserve: Pubkey,
    /// Amount of collateral tokens deposited for an obligation
    pub deposited_tokens: u64,
    /// Market value of collateral
    pub market_value: u64,
    /// Last slot when market value updated
    pub last_update_slot: Slot,
}

/// Create new obligation collateral
pub struct NewObligationCollateralParams {
    /// Deposit reserve address
    pub deposit_reserve: Pubkey,
    /// Current slot
    pub current_slot: Slot,
}

impl ObligationCollateral {
    /// Create new obligation collateral
    pub fn new(params: NewObligationCollateralParams) -> Self {
        let NewObligationCollateralParams {
            deposit_reserve,
            current_slot,
        } = params;

        Self {
            version: PROGRAM_VERSION,
            deposit_reserve,
            deposited_tokens: 0,
            market_value: 0,
            last_update_slot: current_slot,
        }
    }

    /// Return slots elapsed since last update
    fn update_slot(&mut self, slot: Slot) -> u64 {
        // @TODO: checked math?
        let slots_elapsed = slot - self.last_update_slot;
        self.last_update_slot = slot;
        slots_elapsed
    }
}

impl Sealed for ObligationCollateral {}
impl IsInitialized for ObligationCollateral {
    fn is_initialized(&self) -> bool {
        self.version != UNINITIALIZED_VERSION
    }
}

const OBLIGATION_COLLATERAL_LEN: usize = 185; // 1 + 32 + 8 + 8 + 8 + 128
impl Pack for ObligationCollateral {
    const LEN: usize = OBLIGATION_COLLATERAL_LEN;

    fn pack_into_slice(&self, output: &mut [u8]) {
        let output = array_mut_ref![output, 0, OBLIGATION_COLLATERAL_LEN];
        let (version, deposit_reserve, deposited_tokens, market_value, last_update_slot, _padding) =
            mut_array_refs![output, 1, PUBKEY_LEN, 8, 8, 8, 128];

        *version = self.version.to_le_bytes();
        deposit_reserve.copy_from_slice(self.deposit_reserve.as_ref());
        *deposited_tokens = self.deposited_tokens.to_le_bytes();
        *market_value = self.market_value.to_le_bytes();
        *last_update_slot = self.last_update_slot.to_le_bytes();
    }

    /// Unpacks a byte buffer into a [ObligationInfo](struct.ObligationInfo.html).
    fn unpack_from_slice(input: &[u8]) -> Result<Self, ProgramError> {
        let input = array_ref![input, 0, OBLIGATION_COLLATERAL_LEN];
        #[allow(clippy::ptr_offset_with_cast)]
        let (version, deposit_reserve, deposited_tokens, market_value, last_update_slot, _padding) =
            array_refs![input, 1, PUBKEY_LEN, 8, 8, 8, 128];

        Ok(Self {
            version: u8::from_le_bytes(*version),
            deposit_reserve: Pubkey::new_from_array(*deposit_reserve),
            deposited_tokens: u64::from_le_bytes(*deposited_tokens),
            market_value: u64::from_le_bytes(*market_value),
            last_update_slot: u64::from_le_bytes(*last_update_slot),
        })
    }
}
