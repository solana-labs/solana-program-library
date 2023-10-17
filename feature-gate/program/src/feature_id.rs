//! Module for managing feature IDs

use solana_program::{program_error::ProgramError, pubkey::Pubkey};

/// Derives the feature ID from an authority's address and a nonce.
pub fn derive_feature_id(authority: &Pubkey, nonce: u16) -> Result<(Pubkey, u8), ProgramError> {
    Ok(Pubkey::find_program_address(
        &[b"feature", &nonce.to_le_bytes(), authority.as_ref()],
        &crate::id(),
    ))
}
