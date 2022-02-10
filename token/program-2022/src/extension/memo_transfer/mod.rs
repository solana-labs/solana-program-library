use {
    crate::{
        extension::{Extension, ExtensionType, StateWithExtensionsMut},
        pod::PodBool,
        state::Account,
    },
    bytemuck::{Pod, Zeroable},
};

/// Memo Transfer extension instructions
pub mod instruction;

/// Memo Transfer extension processor
pub mod processor;

/// Memo Transfer extension for Accounts
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct MemoTransfer {
    /// Require transfers into this account to be accompanied by a memo
    pub require_incoming_transfer_memos: PodBool,
}
impl Extension for MemoTransfer {
    const TYPE: ExtensionType = ExtensionType::MemoTransfer;
}

/// Determine if a memo is required for transfers into this account
pub fn memo_required(account_state: &StateWithExtensionsMut<Account>) -> bool {
    if let Ok(extension) = account_state.get_extension::<MemoTransfer>() {
        return extension.require_incoming_transfer_memos.into();
    }
    false
}
