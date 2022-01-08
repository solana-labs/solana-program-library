use {
    crate::extension::{AccountType, Extension, ExtensionType},
    bytemuck::{Pod, Zeroable},
};

/// Confidential Transfer Extension instructions
pub mod instruction;

/// Confidential Transfer Extension processor
pub mod processor;

/// Transfer auditor configuration
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct ConfidentialTransferAuditor {
    // TODO: inline `zk_token_program::state::Auditor` here
}

impl Extension for ConfidentialTransferAuditor {
    const TYPE: ExtensionType = ExtensionType::ConfidentialTransferAuditor;
    const ACCOUNT_TYPE: AccountType = AccountType::Mint;
}

/// Confidential account state
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct ConfidentialTransferState {
    // TODO: inline `zk_token_program::state::ZkAccount` here
}

impl Extension for ConfidentialTransferState {
    const TYPE: ExtensionType = ExtensionType::ConfidentialTransferState;
    const ACCOUNT_TYPE: AccountType = AccountType::Account;
}
