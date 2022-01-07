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
pub struct MintConfidentialTransferAuditor {
    // TODO: inline `zk_token_program::state::Auditor` here
}

impl Extension for MintConfidentialTransferAuditor {
    const TYPE: ExtensionType = ExtensionType::MintConfidentialTransferAuditor;
    const ACCOUNT_TYPE: AccountType = AccountType::Mint;
}

/// Confidential account state
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct AccountConfidentialState {
    // TODO: inline `zk_token_program::state::ZkAccount` here
}

impl Extension for AccountConfidentialState {
    const TYPE: ExtensionType = ExtensionType::AccountConfidentialState;
    const ACCOUNT_TYPE: AccountType = AccountType::Account;
}
