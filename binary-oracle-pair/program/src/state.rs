//! State transition types

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

/// Uninitialized version value, all instances are at least version 1
pub const UNINITIALIZED_VERSION: u8 = 0;
/// Initialized pool version
pub const POOL_VERSION: u8 = 1;

/// Program states.
#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct Pool {
    /// Initialized state.
    pub version: u8,

    /// Nonce used in program address.
    pub bump_seed: u8,

    /// Program ID of the tokens
    pub token_program_id: Pubkey,

    /// Account to deposit into
    pub deposit_account: Pubkey,

    /// Mint information for token Pass
    pub token_pass_mint: Pubkey,

    /// Mint information for token Fail
    pub token_fail_mint: Pubkey,

    /// decider key
    pub decider: Pubkey,

    /// mint end slot
    pub mint_end_slot: u64,

    /// decide end slot
    pub decide_end_slot: u64,

    /// decision status
    pub decision: Decision,
}

/// Decision status
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub enum Decision {
    /// Decision was not made
    Undecided,
    /// Decision set at Pass
    Pass,
    /// Decision set at Fail
    Fail,
}

impl Pool {
    /// Length serialized data
    pub const LEN: usize = 179;

    /// Check if Pool already initialized
    pub fn is_initialized(&self) -> bool {
        self.version != UNINITIALIZED_VERSION
    }
}

mod test {
    #[cfg(test)]
    use super::*;

    #[test]
    pub fn test_pool_pack_unpack() {
        let p = Pool {
            version: 1,
            bump_seed: 2,
            token_program_id: Pubkey::new_unique(),
            deposit_account: Pubkey::new_unique(),
            token_pass_mint: Pubkey::new_unique(),
            token_fail_mint: Pubkey::new_unique(),
            decider: Pubkey::new_unique(),
            mint_end_slot: 433,
            decide_end_slot: 5546,
            decision: Decision::Fail,
        };

        let packed = p.try_to_vec().unwrap();

        let unpacked = Pool::try_from_slice(packed.as_slice()).unwrap();

        assert_eq!(p, unpacked);
    }
}
