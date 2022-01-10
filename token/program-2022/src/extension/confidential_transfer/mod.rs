use {
    crate::{
        extension::{Extension, ExtensionType},
        id,
        pod::*,
    },
    bytemuck::{Pod, Zeroable},
    solana_program::pubkey::Pubkey,
    solana_zk_token_sdk::zk_token_elgamal::pod,
};

/// Confidential Transfer Extension instructions
pub mod instruction;

/// Confidential Transfer Extension processor
pub mod processor;

/// Transfer auditor configuration
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct ConfidentialTransferAuditor {
    /// Authority to modify the auditor configuration
    ///
    /// Note that setting an authority of `Pubkey::default()` is the idiomatic way to disable
    /// future changes to the configuration.
    pub authority: Pubkey,

    /// Indicate if newly configured accounts must be approved by the auditor before they may be
    /// used by the user.
    ///
    /// * If `true`, the auditor authority must approve newly configured accounts (see
    ///              `ConfidentialTransferInstruction::ConfigureAccount`)
    /// * If `false`, no approval is required and new accounts may be used immediately
    pub approve_new_accounts: PodBool,

    /// * If non-zero, transfers must include ElGamal cypertext with this public key permitting the
    /// auditor to decode the transfer amount.
    /// * If all zero, auditing is currently disabled.
    pub auditor_pk: pod::ElGamalPubkey,
}

impl Extension for ConfidentialTransferAuditor {
    const TYPE: ExtensionType = ExtensionType::ConfidentialTransferAuditor;
}

/// Confidential account state
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct ConfidentialTransferState {
    /// `true` if this account has been approved for use. All confidential transfer operations for
    /// the account will fail until approval is granted.
    pub approved: PodBool,
    // TODO: inline `zk_token_program::state::ZkAccount` here
}

impl Extension for ConfidentialTransferState {
    const TYPE: ExtensionType = ExtensionType::ConfidentialTransferState;
}

pub(crate) fn get_omnibus_token_address_with_seed(token_mint: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[token_mint.as_ref(), br"confidential_transfer_omnibus"],
        &id(),
    )
}

/// Derive the address of the Omnibus SPL Token account for a given SPL Token mint
///
/// The omnibus account is a central token account that holds all SPL Tokens deposited for
/// confidential transfer by all users
pub fn get_omnibus_token_address(token_mint: &Pubkey) -> Pubkey {
    get_omnibus_token_address_with_seed(token_mint).0
}
