//! Feature Proposal program
#![deny(missing_docs)]
#![forbid(unsafe_code)]

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

pub(crate) fn get_distributor_token_address_with_seed(
    feature_proposal_address: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[&feature_proposal_address.to_bytes(), br"distributor"],
        &id(),
    )
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
pub fn get_distributor_token_address(feature_proposal_address: &Pubkey) -> Pubkey {
    get_distributor_token_address_with_seed(feature_proposal_address).0
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

/// Convert the UI representation of a token amount (using the decimals field defined in its mint)
/// to the raw amount
pub fn ui_amount_to_amount(ui_amount: f64) -> u64 {
    (ui_amount * 10_usize.pow(spl_token::native_mint::DECIMALS as u32) as f64) as u64
}

/// Convert a raw amount to its UI representation (using the decimals field defined in its mint)
pub fn amount_to_ui_amount(amount: u64) -> f64 {
    amount as f64 / 10_usize.pow(spl_token::native_mint::DECIMALS as u32) as f64
}
