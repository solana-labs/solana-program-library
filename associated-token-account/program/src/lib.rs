//! Convention for associating token accounts with a user wallet
#![deny(missing_docs)]
#![forbid(unsafe_code)]

mod entrypoint;
pub mod instruction;
pub mod processor;
pub mod tools;

// Export current SDK types for downstream users building with a different SDK version
pub use solana_program;
use solana_program::{instruction::Instruction, program_pack::Pack, pubkey::Pubkey};

solana_program::declare_id!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");

pub(crate) fn get_associated_token_address_and_bump_seed(
    wallet_address: &Pubkey,
    spl_token_mint_address: &Pubkey,
    program_id: &Pubkey,
) -> (Pubkey, u8) {
    get_associated_token_address_and_bump_seed_internal(
        wallet_address,
        spl_token_mint_address,
        program_id,
        &spl_token::id(),
    )
}

/// Derives the associated token account address for the given wallet address and token mint
pub fn get_associated_token_address(
    wallet_address: &Pubkey,
    spl_token_mint_address: &Pubkey,
) -> Pubkey {
    get_associated_token_address_and_bump_seed(wallet_address, spl_token_mint_address, &id()).0
}

fn get_associated_token_address_and_bump_seed_internal(
    wallet_address: &Pubkey,
    spl_token_mint_address: &Pubkey,
    program_id: &Pubkey,
    token_program_id: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            &wallet_address.to_bytes(),
            &token_program_id.to_bytes(),
            &spl_token_mint_address.to_bytes(),
        ],
        program_id,
    )
}

/// Create an associated token account for the given wallet address and token mint
///
/// Accounts expected by this instruction:
///
///   0. `[writeable,signer]` Funding account (must be a system account)
///   1. `[writeable]` Associated token account address to be created
///   2. `[]` Wallet address for the new associated token account
///   3. `[]` The token mint for the new associated token account
///   4. `[]` System program
///   5. `[]` SPL Token program
///
#[deprecated(
    since = "1.0.4",
    note = "please use `instruction::create_associated_token_account` instead"
)]
pub fn create_associated_token_account(
    funding_address: &Pubkey,
    wallet_address: &Pubkey,
    spl_token_mint_address: &Pubkey,
) -> Instruction {
    instruction::create_associated_token_account(
        funding_address,
        wallet_address,
        spl_token_mint_address,
    )
}
