use solana_program::program_error::ProgramError;
use solana_program::program_pack::{IsInitialized, Pack, Sealed};
use solana_program::pubkey::Pubkey;

use super::UNINITIALIZED_VERSION;

/// Margin Pool
#[repr(C)]
#[derive(Debug, Default, PartialEq)]
pub struct MarginPool {
    /// version of the margin pool
    pub version: u8,

    /// Nonce used in program address.
    /// The program address is created deterministically with the nonce,
    /// swap program id, and swap account pubkey.  This program address has
    /// authority over the swap's token A account, token B account, and pool
    /// token mint.
    pub nonce: u8,

    /// Token LP pool account
    pub token_lp: Pubkey,
    /// Token A - first component of the swap basket
    pub token_a: Pubkey,
    /// Token B - second component of the swap basket
    pub token_b: Pubkey,

    /// Pool tokens are issued when LP tokens are deposited.
    pub pool_mint: Pubkey,

    /// Mint information for token A
    pub token_a_mint: Pubkey,
    /// Mint information for token B
    pub token_b_mint: Pubkey,
    /// Mint information for token LP
    pub token_lp_mint: Pubkey,
    /// token swap pool
    pub token_swap: Pubkey,
    /// Escrow account for A
    pub escrow_a: Pubkey,
    /// Escrow account for B
    pub escrow_b: Pubkey,
    /// Per-slot position fee numerator
    pub position_fee_numerator: u64,
    /// Per-slot position fee denominator
    pub position_fee_denominator: u64,
    /// Fee charged on LP on funds withdrawal numerator
    pub owner_withdraw_fee_numerator: u64,
    /// Fee charged on LP on funds withdrawal denominator
    pub owner_withdraw_fee_denominator: u64,
    /// Part of a position fee transferred to the owner, numerator
    pub owner_position_fee_numerator: u64,
    /// Part of a position fee transferred to the owner, denominator
    pub owner_position_fee_denominator: u64,
    /// Part of a position fee transferred to the position opening host, numerator
    pub host_position_fee_numerator: u64,
    /// Part of a position fee transferred to the position opening host, denominator
    pub host_position_fee_denominator: u64,
    /// Program ID of the tokens being exchanged.
    pub token_program_id: Pubkey,
    /// Program ID of the token swap pool.
    pub token_swap_program_id: Pubkey,
}

impl Pack for MarginPool {
    const LEN: usize = 291;
    fn unpack_from_slice(_input: &[u8]) -> Result<Self, ProgramError> {
        unimplemented!();
    }
    fn pack_into_slice(&self, _output: &mut [u8]) {
        unimplemented!();
    }
}

impl Sealed for MarginPool {}
impl IsInitialized for MarginPool {
    fn is_initialized(&self) -> bool {
        self.version != UNINITIALIZED_VERSION
    }
}
