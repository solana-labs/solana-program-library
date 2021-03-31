#![deny(missing_docs)]

//! A program for creating pools of Solana stakes managed by a Stake-o-Matic

pub mod borsh;
pub mod error;
pub mod instruction;
pub mod processor;
pub mod stake_program;
pub mod state;

#[cfg(not(feature = "no-entrypoint"))]
pub mod entrypoint;

// Export current sdk types for downstream users building with a different sdk version
pub use solana_program;
use solana_program::{program_error::ProgramError, pubkey::Pubkey};

/// Seed for deposit authority seed
pub const AUTHORITY_DEPOSIT: &[u8] = b"deposit";

/// Seed for withdraw authority seed
pub const AUTHORITY_WITHDRAW: &[u8] = b"withdraw";

/// Calculates the authority address
pub fn create_pool_authority_address(
    program_id: &Pubkey,
    stake_pool: &Pubkey,
    authority: &[u8],
    bump_seed: u8,
) -> Result<Pubkey, ProgramError> {
    Pubkey::create_program_address(
        &[&stake_pool.to_bytes()[..32], authority, &[bump_seed]],
        program_id,
    )
    .map_err(|_| crate::error::StakePoolError::InvalidProgramAddress.into())
}

/// Generates seed bump for stake pool authorities
pub fn find_authority_bump_seed(
    program_id: &Pubkey,
    stake_pool: &Pubkey,
    authority: &[u8],
) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[&stake_pool.to_bytes()[..32], authority], program_id)
}
/// Generates stake account address for the validator
pub fn find_stake_address_for_validator(
    program_id: &Pubkey,
    validator: &Pubkey,
    stake_pool: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[&validator.to_bytes()[..32], &stake_pool.to_bytes()[..32]],
        program_id,
    )
}

solana_program::declare_id!("poo1B9L9nR3CrcaziKVYVpRX6A9Y1LAXYasjjfCbApj");
