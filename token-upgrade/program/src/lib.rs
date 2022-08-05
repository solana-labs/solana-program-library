//! Convention for upgrading tokens from one program to another
#![deny(missing_docs)]
#![forbid(unsafe_code)]

mod entrypoint;
pub mod error;
pub mod instruction;
pub mod processor;
pub mod state;

// Export current SDK types for downstream users building with a different SDK version
pub use solana_program;
use solana_program::pubkey::Pubkey;

solana_program::declare_id!("TokuPsq2wbFopRYJ44C3Gcg63TzG7z951vTVU3eYarC");

/// Get the upgrade factory address for a source mint
pub fn get_factory_address(source_mint_address: &Pubkey, program_id: &Pubkey) -> Pubkey {
    get_factory_address_and_bump_seed(source_mint_address, program_id).0
}

pub(crate) fn get_factory_address_and_bump_seed(
    source_mint_address: &Pubkey,
    program_id: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[&source_mint_address.to_bytes()], program_id)
}

const FACTORY_MINT_AUTHORITY_SEED: &[u8] = b"mint-authority";

/// Get the upgrade factory mint authority for a source mint
pub fn get_factory_mint_authority_address(
    source_mint_address: &Pubkey,
    program_id: &Pubkey,
) -> Pubkey {
    get_factory_mint_authority_address_and_bump_seed(source_mint_address, program_id).0
}

pub(crate) fn get_factory_mint_authority_address_and_bump_seed(
    source_mint_address: &Pubkey,
    program_id: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[FACTORY_MINT_AUTHORITY_SEED, &source_mint_address.to_bytes()],
        program_id,
    )
}
