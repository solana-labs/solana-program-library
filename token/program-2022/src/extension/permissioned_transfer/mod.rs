use {
    crate::{
        extension::{Extension, ExtensionType},
        pod::OptionalNonZeroPubkey,
    },
    bytemuck::{Pod, Zeroable},
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
    pub permissioned_transfer_program_id: OptionalNonZeroPubkey,
}
impl Extension for PermissionedTransfer {
    const TYPE: ExtensionType = ExtensionType::PermissionedTransfer;
}
