//! Convention for associating token accounts with a user wallet
#![deny(missing_docs)]
#![forbid(unsafe_code)]

mod entrypoint;
pub mod processor;
pub mod state;
mod error;
pub mod instruction;
mod tools;

// Export current SDK types for downstream users building with a different SDK version
pub use solana_program;
use solana_program::{
    pubkey::Pubkey,
};

solana_program::declare_id!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");

pub(crate) fn get_multi_delegate_address_and_bump_seed(
    token_account_owner: &Pubkey,
    token_account_address: &Pubkey,
    program_id: &Pubkey,
) -> (Pubkey, u8) {
    get_multi_delegate_address_and_bump_seed_internal(
        token_account_owner,
        token_account_address,
        program_id,
    )
}

/// Derives the associated token account address for the given wallet address and token mint
pub fn get_multi_delegate_address(
    token_account_owner: &Pubkey,
    token_account_address: &Pubkey,
) -> Pubkey {
    get_multi_delegate_address_and_bump_seed(token_account_owner, token_account_address, &id()).0
}

/// The thing but internal
pub fn get_multi_delegate_address_and_bump_seed_internal(
    token_account_owner: &Pubkey,
    token_account_address: &Pubkey,
    program_id: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            token_account_owner.as_ref(),
            token_account_address.as_ref(),
        ],
        program_id,
    )
}
