//! State transition types

//use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use solana_program::{
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};

/// Uninitialized version value, all instances are at least version 1
pub const UNINITIALIZED_VERSION: u8 = 0;

/// Program states.
#[repr(C)]
#[derive(Debug, Default, PartialEq)]
pub struct Pool {
    /// Initialized state.
    pub version: u8,

    /// Nonce used in program address.
    pub nonce: u8,

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
    const LEN: usize = 1;

    fn pack_into_slice(&self, _output: &mut [u8]) {
        unimplemented!();
    }
    fn unpack_from_slice(_input: &[u8]) -> Result<Self, ProgramError> {
        unimplemented!();
    }
}
