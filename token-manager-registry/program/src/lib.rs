//! A program demonstrating how to register a token manager program
#![deny(missing_docs)]
#![forbid(unsafe_code)]

mod entrypoint;
pub mod processor;

pub use solana_program;
use solana_program::{declare_id, pubkey::Pubkey};

/// Generates the registration address for a mint
///
/// The registration address defines the program id to be used for the transfer
/// resolution
pub fn find_manager_registration_address(program_id: &Pubkey, mint_address: &Pubkey) -> Pubkey {
    find_manager_registration_address_internal(program_id, mint_address).0
}

pub(crate) fn find_manager_registration_address_internal(
    program_id: &Pubkey,
    mint_address: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[mint_address.as_ref()], program_id)
}

declare_id!("TMreguGXkTM37TkytTJ4mQMgEBaYSBajFsuFFHL25DJ");
