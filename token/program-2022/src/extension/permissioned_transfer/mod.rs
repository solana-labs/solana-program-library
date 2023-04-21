use {
    crate::{
        extension::{BaseState, BaseStateWithExtensions, Extension, ExtensionType},
        pod::OptionalNonZeroPubkey,
    },
    bytemuck::{Pod, Zeroable},
    solana_program::pubkey::Pubkey,
};

/// Instructions for the PermissionedTransfer extension
pub mod instruction;
/// Instruction processor for the PermissionedTransfer extension
pub mod processor;

/// Close authority extension data for mints.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct PermissionedTransfer {
    /// Authority that can set the permissioned transfer program id
    pub authority: OptionalNonZeroPubkey,
    /// Program that authorizes the transfer
    pub program_id: OptionalNonZeroPubkey,
}

/// Indicates that the tokens from this account belong to a permissioned-transfer mint
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
#[repr(transparent)]
pub struct PermissionedTransferAccount;

impl Extension for PermissionedTransfer {
    const TYPE: ExtensionType = ExtensionType::PermissionedTransfer;
}

impl Extension for PermissionedTransferAccount {
    const TYPE: ExtensionType = ExtensionType::PermissionedTransferAccount;
}

/// Attempts to get the permissioned transfer program id from the TLV data, returning
/// None if the extension is not found
pub fn get_permissioned_transfer_program_id<S: BaseState, BSE: BaseStateWithExtensions<S>>(
    state: &BSE,
) -> Option<Pubkey> {
    state
        .get_extension::<PermissionedTransfer>()
        .ok()
        .and_then(|e| Option::<Pubkey>::from(e.program_id))
}
