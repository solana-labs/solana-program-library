//! Convention for upgrading tokens from one program to another
#![deny(missing_docs)]
#![forbid(unsafe_code)]

mod entrypoint;
pub mod error;
pub mod instruction;
pub mod processor;

// Export current SDK types for downstream users building with a different SDK version
pub use solana_program;
use solana_program::pubkey::Pubkey;

solana_program::declare_id!("TokuPsq2wbFopRYJ44C3Gcg63TzG7z951vTVU3eYarC");

const TOKEN_UPGRADE_AUTHORITY_SEED: &[u8] = b"token-account-authority";

/// Get the upgrade token account authority
pub fn get_token_upgrade_authority_address(
    source_mint: &Pubkey,
    destination_mint: &Pubkey,
    program_id: &Pubkey,
) -> Pubkey {
    get_token_upgrade_authority_address_and_bump_seed(source_mint, destination_mint, program_id).0
}

pub(crate) fn get_token_upgrade_authority_address_and_bump_seed(
    source_mint: &Pubkey,
    destination_mint: &Pubkey,
    program_id: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            TOKEN_UPGRADE_AUTHORITY_SEED,
            source_mint.as_ref(),
            destination_mint.as_ref(),
        ],
        program_id,
    )
}
