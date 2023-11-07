//! Convention for upgrading tokens from one program to another
#![deny(missing_docs)]
#![forbid(unsafe_code)]

mod entrypoint;
pub mod error;
pub mod instruction;
pub mod processor;

// Export current SDK types for downstream users building with a different SDK
// version
pub use solana_program;
use solana_program::pubkey::Pubkey;

solana_program::declare_id!("TkupDoNseygccBCjSsrSpMccjwHfTYwcrjpnDSrFDhC");

const TOKEN_ESCROW_AUTHORITY_SEED: &[u8] = b"token-escrow-authority";

/// Get the upgrade token account authority
pub fn get_token_upgrade_authority_address(
    original_mint: &Pubkey,
    new_mint: &Pubkey,
    program_id: &Pubkey,
) -> Pubkey {
    get_token_upgrade_authority_address_and_bump_seed(original_mint, new_mint, program_id).0
}

pub(crate) fn get_token_upgrade_authority_address_and_bump_seed(
    original_mint: &Pubkey,
    new_mint: &Pubkey,
    program_id: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &collect_token_upgrade_authority_seeds(original_mint, new_mint),
        program_id,
    )
}

pub(crate) fn collect_token_upgrade_authority_seeds<'a>(
    original_mint: &'a Pubkey,
    new_mint: &'a Pubkey,
) -> [&'a [u8]; 3] {
    [
        TOKEN_ESCROW_AUTHORITY_SEED,
        original_mint.as_ref(),
        new_mint.as_ref(),
    ]
}

pub(crate) fn collect_token_upgrade_authority_signer_seeds<'a>(
    original_mint: &'a Pubkey,
    new_mint: &'a Pubkey,
    bump_seed: &'a [u8],
) -> [&'a [u8]; 4] {
    [
        TOKEN_ESCROW_AUTHORITY_SEED,
        original_mint.as_ref(),
        new_mint.as_ref(),
        bump_seed,
    ]
}
