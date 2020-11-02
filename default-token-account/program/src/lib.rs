//! Manages the default token account for wallets that support SPL Token
#![deny(missing_docs)]
#![forbid(unsafe_code)]

mod entrypoint;
pub mod error;
pub mod instruction;
pub mod processor;

// Export current sdk types for downstream users building with a different sdk version
pub use solana_program;

solana_program::declare_id!("5p1zaZPmaL745KK5xi1MVj7QsMjWFBR6Q4WzYC5gJxSj");

use solana_program::pubkey::Pubkey;

/// Returns the default token account address for the given token mint and wallet address
pub fn get_default_token_account_address(
    default_token_account_program_id: &Pubkey,
    token_program_id: &Pubkey,
    token_mint_address: &Pubkey,
    wallet_address: &Pubkey,
) -> Pubkey {
    get_default_token_account_address_and_bump_seed(
        default_token_account_program_id,
        token_program_id,
        token_mint_address,
        wallet_address,
    )
    .0
}

pub(crate) fn get_default_token_account_address_and_bump_seed(
    default_token_account_program_id: &Pubkey,
    token_program_id: &Pubkey,
    token_mint_address: &Pubkey,
    wallet_address: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            &token_program_id.to_bytes(),
            &token_mint_address.to_bytes(),
            &wallet_address.to_bytes(),
        ],
        default_token_account_program_id,
    )
}
