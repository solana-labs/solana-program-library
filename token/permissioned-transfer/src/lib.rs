//! Crate defining an interface for performing permissioned transfers, where the
//! token program calls into a separate program with additional accounts to be
//! sure that a transfer has accomplished all required preconditions.

#![allow(clippy::integer_arithmetic)]
#![deny(missing_docs)]
#![cfg_attr(not(test), forbid(unsafe_code))]

pub mod error;
pub mod inline_spl_token;
pub mod instruction;
pub mod pod;
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

/// Seed for the state
const EXTRA_ACCOUNT_METAS_SEED: &[u8] = b"extra-account-metas";

/// Get the validate state address
pub fn get_extra_account_metas_address(mint: &Pubkey, program_id: &Pubkey) -> Pubkey {
    get_extra_account_metas_address_and_bump_seed(mint, program_id).0
}

pub(crate) fn get_extra_account_metas_address_and_bump_seed(
    mint: &Pubkey,
    program_id: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(&collect_extra_account_metas_seeds(mint), program_id)
}

pub(crate) fn collect_extra_account_metas_seeds(mint: &Pubkey) -> [&[u8]; 2] {
    [EXTRA_ACCOUNT_METAS_SEED, mint.as_ref()]
}

pub(crate) fn collect_extra_account_metas_signer_seeds<'a>(
    mint: &'a Pubkey,
    bump_seed: &'a [u8],
) -> [&'a [u8]; 3] {
    [EXTRA_ACCOUNT_METAS_SEED, mint.as_ref(), bump_seed]
}

solana_program::declare_id!("pERmRFhRmg9JaJdsocrUnLLigHXrwWTxBu2SafwK2cd");
