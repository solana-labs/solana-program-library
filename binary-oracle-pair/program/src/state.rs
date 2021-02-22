//! State transition types

use solana_program::{
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};
use std::mem::size_of;

/// Uninitialized version value, all instances are at least version 1
pub const UNINITIALIZED_VERSION: u8 = 0;

/// Program states.
#[repr(C)]
#[derive(Debug, Default, PartialEq, Clone)]
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

    /// decision boolean
    pub decision: Option<bool>,
}

impl Sealed for Pool {}
impl IsInitialized for Pool {
    fn is_initialized(&self) -> bool {
        self.version != UNINITIALIZED_VERSION
    }
}

impl Pack for Pool {
    const LEN: usize = size_of::<Self>() + 1; // +1 because of Option<> type

    fn pack_into_slice(&self, output: &mut [u8]) {
        output[0] = self.version;
        output[1] = self.bump_seed;
        output[2..34].copy_from_slice(&self.token_program_id.to_bytes());
        output[34..66].copy_from_slice(&self.deposit_account.to_bytes());
        output[66..98].copy_from_slice(&self.token_pass_mint.to_bytes());
        output[98..130].copy_from_slice(&self.token_fail_mint.to_bytes());
        output[130..162].copy_from_slice(&self.decider.to_bytes());
        output[162..].copy_from_slice(&[
            if self.decision.is_some() { 1 } else { 0 },
            self.decision.unwrap_or(false) as u8,
        ]);
    }
    fn unpack_from_slice(input: &[u8]) -> Result<Self, ProgramError> {
        Ok(Pool {
            version: input[0],
            bump_seed: input[1],
            token_program_id: Pubkey::new(&input[2..34]),
            deposit_account: Pubkey::new(&input[34..66]),
            token_pass_mint: Pubkey::new(&input[66..98]),
            token_fail_mint: Pubkey::new(&input[98..130]),
            decider: Pubkey::new(&input[130..162]),
            decision: if input[162] == 0 {
                None
            } else {
                if input[163] == 1 {
                    Some(true)
                } else {
                    Some(false)
                }
            },
        })
    }
}

mod test {
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
            decision: Some(false),
        };

        let mut packed = vec![0u8; Pool::LEN];
        Pool::pack(p.clone(), packed.as_mut_slice()).unwrap();

        let unpacked = Pool::unpack(packed.as_slice()).unwrap();

        assert_eq!(p, unpacked);
    }
}
