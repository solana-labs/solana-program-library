//! Crate defining SPL Interface Base utils

#![deny(missing_docs)]
#![cfg_attr(not(test), forbid(unsafe_code))]

pub mod instruction;
pub mod state;

pub use solana_program::pubkey::Pubkey;

/// Namespace for the base utils
pub const NAMESPACE: &str = "spl_interface_base";

/// Seed for the state
const EXTRA_ACCOUNT_METAS_SEED: &[u8] = b"extra-account-metas";

/// Get the state address PDA
pub fn get_extra_account_metas_address(mint: &Pubkey, program_id: &Pubkey) -> Pubkey {
    get_extra_account_metas_address_and_bump_seed(mint, program_id).0
}

/// Function used by programs implementing the interface, when creating the PDA,
/// to also get the bump seed
pub fn get_extra_account_metas_address_and_bump_seed(
    mint: &Pubkey,
    program_id: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(&collect_extra_account_metas_seeds(mint), program_id)
}

/// Function used by programs implementing the interface, when creating the PDA,
/// to get all of the PDA seeds
pub fn collect_extra_account_metas_seeds(mint: &Pubkey) -> [&[u8]; 2] {
    [EXTRA_ACCOUNT_METAS_SEED, mint.as_ref()]
}

/// Function used by programs implementing the interface, when creating the PDA,
/// to sign for the PDA
pub fn collect_extra_account_metas_signer_seeds<'a>(
    mint: &'a Pubkey,
    bump_seed: &'a [u8],
) -> [&'a [u8]; 3] {
    [EXTRA_ACCOUNT_METAS_SEED, mint.as_ref(), bump_seed]
}

/// Get the slice corresponding to the given start and end range
pub fn get_emit_slice(data: &[u8], start: Option<u64>, end: Option<u64>) -> Option<&[u8]> {
    let start: usize = start.unwrap_or(0) as usize;
    let end = end.map(|x| x as usize).unwrap_or(data.len());
    data.get(start..end)
}
