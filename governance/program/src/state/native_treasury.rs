//! Native treasury account

use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use solana_program::pubkey::Pubkey;
use spl_governance_tools::account::AccountMaxSize;

/// Treasury account
/// The account has no data and can be used as a payer for instruction signed by Governance PDAs or as a native SOL treasury
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub struct NativeTreasury {}

impl AccountMaxSize for NativeTreasury {
    fn get_max_size(&self) -> Option<usize> {
        Some(0)
    }
}

/// Returns NativeTreasury PDA seeds
pub fn get_native_treasury_address_seeds(governance: &Pubkey) -> [&[u8]; 2] {
    [b"native-treasury", governance.as_ref()]
}

/// Returns NativeTreasury PDA address
pub fn get_native_treasury_address(program_id: &Pubkey, governance: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&get_native_treasury_address_seeds(governance), program_id).0
}
