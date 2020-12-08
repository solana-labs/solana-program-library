//! State transition types

use solana_program::pubkey::Pubkey;
use solana_program::program_pack::{IsInitialized, Pack, Sealed};
use solana_program::program_error::ProgramError;


/// Possible program states
#[repr(C)]
#[derive(Debug, PartialEq)]
pub enum State {
    /// starting state
    Uninitialized,
    /// margin pool
    MarginPool(MarginPool),
    /// Position
    Position(Position),
}

impl Pack for State {
    const LEN: usize = 291;
    fn unpack_from_slice(_input: &[u8]) -> Result<Self, ProgramError> {
        unimplemented!();
    }
    fn pack_into_slice(&self, _output: &mut [u8]) {
        unimplemented!();
    }
}

impl Sealed for State {}
impl IsInitialized for State {
    fn is_initialized(&self) -> bool {
        unimplemented!();
    }
}

/// Margin Pool
#[repr(C)]
#[derive(Debug, Default, PartialEq)]
pub struct MarginPool {
    /// Nonce used in program address.
    /// The program address is created deterministically with the nonce,
    /// swap program id, and swap account pubkey.  This program address has
    /// authority over the swap's token A account, token B account, and pool
    /// token mint.
    pub nonce: u8,

    /// Program ID of the tokens being exchanged.
    pub token_program_id: Pubkey,

    /// Program ID of the token swap pool.
    pub token_swap_program_id: Pubkey,

    /// Token LP pool account
    pub token_lp: Pubkey,

    /// Token A
    pub token_a: Pubkey,
    /// Token B
    pub token_b: Pubkey,

    /// Pool tokens are issued when LP tokens are deposited.
    pub pool_mint: Pubkey,

    /// Mint information for token A
    pub token_a_mint: Pubkey,
    /// Mint information for token B
    pub token_b_mint: Pubkey,

    /// Mint information for token LP
    pub token_lp_mint: Pubkey,

    // TBD
    // /// Debt Pool A
    // pub debt_mint_a: Pubkey,

    // /// Debt Pool B
    // pub debt_mint_b: Pubkey,
}

/// Position state
#[repr(C)]
#[derive(Debug, Default, PartialEq)]
pub struct Position {
    pub slot: Slot,
    pub collateral_amount: u64,
    pub size: u64,
    pub mint: Pubkey,
}
