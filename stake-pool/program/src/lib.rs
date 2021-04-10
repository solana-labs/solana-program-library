#![deny(missing_docs)]

//! A program for creating and managing pools of stake

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
use solana_program::pubkey::Pubkey;

/// Seed for deposit authority seed
const AUTHORITY_DEPOSIT: &[u8] = b"deposit";

/// Seed for withdraw authority seed
const AUTHORITY_WITHDRAW: &[u8] = b"withdraw";

/// Generates the deposit authority program address for the stake pool
pub fn find_deposit_authority_program_address(
    program_id: &Pubkey,
    stake_pool_address: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[&stake_pool_address.to_bytes()[..32], AUTHORITY_DEPOSIT],
        program_id,
    )
}

/// Generates the withdraw authority program address for the stake pool
pub fn find_withdraw_authority_program_address(
    program_id: &Pubkey,
    stake_pool_address: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[&stake_pool_address.to_bytes()[..32], AUTHORITY_WITHDRAW],
        program_id,
    )
}

/// Generates the stake program address for a validator's vote account
pub fn find_stake_program_address(
    program_id: &Pubkey,
    vote_account_address: &Pubkey,
    stake_pool_address: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            &vote_account_address.to_bytes()[..32],
            &stake_pool_address.to_bytes()[..32],
        ],
        program_id,
    )
}

solana_program::declare_id!("poo1B9L9nR3CrcaziKVYVpRX6A9Y1LAXYasjjfCbApj");
