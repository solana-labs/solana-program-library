#![allow(missing_docs)]

use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};

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
    const LEN: usize = 97;
    fn pack_into_slice(&self, output: &mut [u8]) {
        let output = array_mut_ref![output, 0, 97];
        let (version, collateral_amount, size, mint, margin_pool, last_update_slot, fee_per_slot) =
            mut_array_refs![output, 1, 8, 8, 32, 32, 8, 8];
        *version = self.version.to_le_bytes();
        *collateral_amount = self.collateral_amount.to_le_bytes();
        *size = self.size.to_le_bytes();
        mint.copy_from_slice(self.mint.as_ref());
        margin_pool.copy_from_slice(self.margin_pool.as_ref());
        *last_update_slot = self.last_update_slot.to_le_bytes();
        *fee_per_slot = self.fee_per_slot.to_le_bytes();
    }
    fn unpack_from_slice(input: &[u8]) -> Result<Self, ProgramError> {
        let input = array_ref![input, 0, 97];

        let (version, collateral_amount, size, mint, margin_pool, last_update_slot, fee_per_slot) =
            array_refs![input, 1, 8, 8, 32, 32, 8, 8];

        Ok(Self {
            version: u8::from_le_bytes(*version),
            collateral_amount: u64::from_le_bytes(*collateral_amount),
            size: u64::from_le_bytes(*size),
            mint: Pubkey::new_from_array(*mint),
            margin_pool: Pubkey::new_from_array(*margin_pool),
            last_update_slot: u64::from_le_bytes(*last_update_slot),
            fee_per_slot: u64::from_le_bytes(*fee_per_slot),
        })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pack_position() {
        let fees = Position {
            version: 1,
            collateral_amount: 10,
            size: 20,
            mint: Pubkey::new_from_array([1u8; 32]),
            margin_pool: Pubkey::new_from_array([2u8; 32]),
            last_update_slot: 13,
            fee_per_slot: 14,
        };

        let mut packed = [0u8; Position::LEN];
        fees.pack_into_slice(&mut packed);

        let unpacked = Position::unpack_from_slice(&packed).unwrap();

        assert_eq!(fees, unpacked);
    }
}
