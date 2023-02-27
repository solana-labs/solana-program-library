//! Crate defining an interface for performing permissioned transfers, where the
//! token program calls into a separate program with additional accounts to be
//! sure that a transfer has accomplished all required preconditions.

#![allow(clippy::integer_arithmetic)]
#![deny(missing_docs)]
#![cfg_attr(not(test), forbid(unsafe_code))]

pub mod error;
pub mod instruction;
pub mod processor;
pub mod state;
pub mod tlv;

#[cfg(not(feature = "no-entrypoint"))]
mod entrypoint;

// Export current sdk types for downstream users building with a different sdk version
pub use solana_program;
use solana_program::pubkey::Pubkey;

/// Namespace for all programs implementing permissioned-transfer
pub const NAMESPACE: &str = "permissioned-transfer-interface";

/// Size for discriminator in account and instruction data
pub const DISCRIMINATOR_LENGTH: usize = 8;

/// Seed for the validation state
const VALIDATE_STATE_SEED: &[u8] = b"validate-state";

/// Get the validate state address
pub fn get_validate_state_address(
    mint: &Pubkey,
    program_id: &Pubkey,
) -> Pubkey {
    get_validate_state_address_and_bump_seed(mint, program_id).0
}

pub(crate) fn get_validate_state_address_and_bump_seed(
    mint: &Pubkey,
    program_id: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &collect_validate_state_seeds(mint),
        program_id,
    )
}

pub(crate) fn collect_validate_state_seeds<'a>(
    mint: &'a Pubkey,
) -> [&'a [u8]; 2] {
    [
        VALIDATE_STATE_SEED,
        mint.as_ref(),
    ]
}

pub(crate) fn collect_validate_state_signer_seeds<'a>(
    mint: &'a Pubkey,
    bump_seed: &'a [u8],
) -> [&'a [u8]; 3] {
    [
        VALIDATE_STATE_SEED,
        mint.as_ref(),
        bump_seed,
    ]
}
