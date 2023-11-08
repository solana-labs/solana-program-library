//! Token Wrap program
#![deny(missing_docs)]
#![forbid(unsafe_code)]

mod entrypoint;
pub mod instruction;
pub mod processor;
pub mod state;

// Export current SDK types for downstream users building with a different SDK
// version
pub use solana_program;
use solana_program::pubkey::Pubkey;

solana_program::declare_id!("TwRapQCDhWkZRrDaHfZGuHxkZ91gHDRkyuzNqeU5MgR");

const WRAPPED_MINT_SEED: &[u8] = br"mint";

pub(crate) fn get_wrapped_mint_address_with_seed(
    unwrapped_mint: &Pubkey,
    wrapped_token_program_id: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &get_wrapped_mint_seeds(unwrapped_mint, wrapped_token_program_id),
        &id(),
    )
}

pub(crate) fn get_wrapped_mint_seeds<'a>(
    unwrapped_mint: &'a Pubkey,
    wrapped_token_program_id: &'a Pubkey,
) -> [&'a [u8]; 3] {
    [
        WRAPPED_MINT_SEED,
        unwrapped_mint.as_ref(),
        wrapped_token_program_id.as_ref(),
    ]
}

pub(crate) fn _get_wrapped_mint_signer_seeds<'a>(
    unwrapped_mint: &'a Pubkey,
    wrapped_token_program_id: &'a Pubkey,
    bump_seed: &'a [u8],
) -> [&'a [u8]; 4] {
    [
        WRAPPED_MINT_SEED,
        unwrapped_mint.as_ref(),
        wrapped_token_program_id.as_ref(),
        bump_seed,
    ]
}

/// Derive the SPL Token wrapped mint address associated with an unwrapped mint
pub fn get_wrapped_mint_address(
    unwrapped_mint: &Pubkey,
    wrapped_token_program_id: &Pubkey,
) -> Pubkey {
    get_wrapped_mint_address_with_seed(unwrapped_mint, wrapped_token_program_id).0
}

const WRAPPED_MINT_AUTHORITY_SEED: &[u8] = br"authority";

pub(crate) fn get_wrapped_mint_authority_seeds(wrapped_mint: &Pubkey) -> [&[u8]; 2] {
    [WRAPPED_MINT_AUTHORITY_SEED, wrapped_mint.as_ref()]
}

pub(crate) fn _get_wrapped_mint_authority_signer_seeds<'a>(
    wrapped_mint: &'a Pubkey,
    bump_seed: &'a [u8],
) -> [&'a [u8]; 3] {
    [
        WRAPPED_MINT_AUTHORITY_SEED,
        wrapped_mint.as_ref(),
        bump_seed,
    ]
}

pub(crate) fn get_wrapped_mint_authority_with_seed(wrapped_mint: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&get_wrapped_mint_authority_seeds(wrapped_mint), &id())
}

/// Derive the SPL Token wrapped mint authority address
pub fn get_wrapped_mint_authority(wrapped_mint: &Pubkey) -> Pubkey {
    get_wrapped_mint_authority_with_seed(wrapped_mint).0
}

const WRAPPED_MINT_BACKPOINTER_SEED: &[u8] = br"backpointer";

pub(crate) fn get_wrapped_mint_backpointer_address_seeds(wrapped_mint: &Pubkey) -> [&[u8]; 2] {
    [WRAPPED_MINT_BACKPOINTER_SEED, wrapped_mint.as_ref()]
}

pub(crate) fn _get_wrapped_mint_backpointer_address_signer_seeds<'a>(
    wrapped_mint: &'a Pubkey,
    bump_seed: &'a [u8],
) -> [&'a [u8]; 3] {
    [
        WRAPPED_MINT_BACKPOINTER_SEED,
        wrapped_mint.as_ref(),
        bump_seed,
    ]
}

pub(crate) fn get_wrapped_mint_backpointer_address_with_seed(
    wrapped_mint: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &get_wrapped_mint_backpointer_address_seeds(wrapped_mint),
        &id(),
    )
}

/// Derive the SPL Token wrapped mint backpointer address
pub fn get_wrapped_mint_backpointer_address(wrapped_mint: &Pubkey) -> Pubkey {
    get_wrapped_mint_backpointer_address_with_seed(wrapped_mint).0
}
