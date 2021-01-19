#![allow(missing_docs)]

use solana_program::program_error::ProgramError;
use solana_program::program_pack::{IsInitialized, Pack, Sealed};
use solana_program::{epoch_schedule::Slot, pubkey::Pubkey};

use super::UNINITIALIZED_VERSION;

/// Position state
#[repr(C)]
#[derive(Debug, Default, PartialEq)]
pub struct Position {
    /// version of the margin pool, 0 for uninitalized state
    pub version: u8,
    /// Collateral amount provided by the user for this margin position
    pub collateral_amount: u64,
    /// Position size. How much in liquidity tokens was spend on buying position tokens
    pub size: u64,
    ///
    pub mint: Pubkey,
    /// Margin pool account this position was opened with
    pub margin_pool: Pubkey,
    /// Slot when position was opened or funded or reduced, used to calculate outstading fee
    pub last_update_slot: Slot,
    /// How much is charged each slot in liquidity tokens while position is opened, =position_fee * amount
    pub fee_per_slot: u64,
}

impl Pack for Position {
    const LEN: usize = 291;
    fn unpack_from_slice(_input: &[u8]) -> Result<Self, ProgramError> {
        unimplemented!();
    }
    fn pack_into_slice(&self, _output: &mut [u8]) {
        unimplemented!();
    }
}

impl Sealed for Position {}
impl IsInitialized for Position {
    fn is_initialized(&self) -> bool {
        self.version != UNINITIALIZED_VERSION
    }
}

impl Position {
    pub fn charge_yield(&self) -> bool {
        unimplemented!();
    }
}
