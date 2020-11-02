//! Conventions for associating accounts with a primary account (such as a user wallet)
#![deny(missing_docs)]
#![forbid(unsafe_code)]

mod entrypoint;
pub mod instruction;
pub mod processor;
pub mod token;

// Export current sdk types for downstream users building with a different sdk version
pub use solana_program;
use solana_program::pubkey::Pubkey;

solana_program::declare_id!("5p1zaZPmaL745KK5xi1MVj7QsMjWFBR6Q4WzYC5gJxSj");

/// Derives the associated address and bump seed for the `primary_account_address` and
/// `associated_account_program_id`.  Depending on the `associated_account_program_id`,
/// `additional_addresses` may need to be provided to complete the derivation.
///
pub fn get_associated_address_and_bump_seed(
    primary_account_address: &Pubkey,
    associated_account_program_id: &Pubkey,
    additional_addresses: &[&Pubkey],
) -> (Pubkey, u8) {
    crate::processor::get_associated_address_and_bump_seed_with_id(
        primary_account_address,
        associated_account_program_id,
        additional_addresses,
        &id(),
    )
}
