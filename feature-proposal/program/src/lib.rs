//! Feature Proposal program
#![deny(missing_docs)]
#![forbid(unsafe_code)]

pub mod borsh_utils;
mod entrypoint;
pub mod instruction;
pub mod processor;
pub mod state;

// Export current SDK types for downstream users building with a different SDK version
pub use solana_program;
use solana_program::{program_pack::Pack, pubkey::Pubkey};

solana_program::declare_id!("Feat1YXHhH6t1juaWF74WLcfv4XoNocjXA6sPWHNgAse");

pub(crate) fn get_mint_address_with_seed(feature_proposal_address: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[&feature_proposal_address.to_bytes(), br"mint"], &id())
}

pub(crate) fn get_delivery_token_address_with_seed(
    feature_proposal_address: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[&feature_proposal_address.to_bytes(), br"delivery"], &id())
}

pub(crate) fn get_acceptance_token_address_with_seed(
    feature_proposal_address: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[&feature_proposal_address.to_bytes(), br"acceptance"],
        &id(),
    )
}

pub(crate) fn get_feature_id_address_with_seed(feature_proposal_address: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[&feature_proposal_address.to_bytes(), br"feature-id"],
        &id(),
    )
}

/// Derive the SPL Token mint address associated with a feature proposal
pub fn get_mint_address(feature_proposal_address: &Pubkey) -> Pubkey {
    get_mint_address_with_seed(feature_proposal_address).0
}

/// Derive the SPL Token token address associated with a feature proposal that receives the initial
/// minted tokens
pub fn get_delivery_token_address(feature_proposal_address: &Pubkey) -> Pubkey {
    get_delivery_token_address_with_seed(feature_proposal_address).0
}

/// Derive the SPL Token token address associated with a feature proposal that users send their
/// tokens to accept the proposal
pub fn get_acceptance_token_address(feature_proposal_address: &Pubkey) -> Pubkey {
    get_acceptance_token_address_with_seed(feature_proposal_address).0
}

/// Derive the feature id address associated with the feature proposal
pub fn get_feature_id_address(feature_proposal_address: &Pubkey) -> Pubkey {
    get_feature_id_address_with_seed(feature_proposal_address).0
}
