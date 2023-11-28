#[cfg(feature = "serde-traits")]
use serde::{Deserialize, Serialize};
use {
    crate::extension::{Extension, ExtensionType},
    bytemuck::{Pod, Zeroable},
    spl_pod::optional_keys::OptionalNonZeroPubkey,
};

/// Instructions for the GroupMemberPointer extension
pub mod instruction;
/// Instruction processor for the GroupMemberPointer extension
pub mod processor;

/// Group member pointer extension data for mints.
#[repr(C)]
#[cfg_attr(feature = "serde-traits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-traits", serde(rename_all = "camelCase"))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct GroupMemberPointer {
    /// Authority that can set the member address
    pub authority: OptionalNonZeroPubkey,
    /// Account address that holds the member
    pub member_address: OptionalNonZeroPubkey,
}

impl Extension for GroupMemberPointer {
    const TYPE: ExtensionType = ExtensionType::GroupMemberPointer;
}
