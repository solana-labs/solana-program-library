use {
    crate::{
        extension::{BaseState, BaseStateWithExtensions, Extension, ExtensionType},
        pod::OptionalNonZeroPubkey,
    },
    bytemuck::{Pod, Zeroable},
    solana_program::pubkey::Pubkey,
};

/// Instructions for the TransferHook extension
pub mod instruction;
/// Instruction processor for the TransferHook extension
pub mod processor;

/// Close authority extension data for mints.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct TransferHook {
    /// Authority that can set the transfer hook program id
    pub authority: OptionalNonZeroPubkey,
    /// Program that authorizes the transfer
    pub program_id: OptionalNonZeroPubkey,
}

/// Indicates that the tokens from this account belong to a mint with a transfer hook
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
#[repr(transparent)]
pub struct TransferHookAccount;

impl Extension for TransferHook {
    const TYPE: ExtensionType = ExtensionType::TransferHook;
}

impl Extension for TransferHookAccount {
    const TYPE: ExtensionType = ExtensionType::TransferHookAccount;
}

/// Attempts to get the transfer hook program id from the TLV data, returning
/// None if the extension is not found
pub fn get_program_id<S: BaseState, BSE: BaseStateWithExtensions<S>>(
    state: &BSE,
) -> Option<Pubkey> {
    state
        .get_extension::<TransferHook>()
        .ok()
        .and_then(|e| Option::<Pubkey>::from(e.program_id))
}
