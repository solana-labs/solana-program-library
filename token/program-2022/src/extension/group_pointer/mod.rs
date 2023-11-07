#[cfg(feature = "serde-traits")]
use serde::{Deserialize, Serialize};
use {
    crate::extension::{Extension, ExtensionType},
    bytemuck::{Pod, Zeroable},
    spl_pod::optional_keys::OptionalNonZeroPubkey,
};

/// Instructions for the GroupPointer extension
pub mod instruction;
/// Instruction processor for the GroupPointer extension
pub mod processor;

/// Group pointer extension data for mints.
#[repr(C)]
#[cfg_attr(feature = "serde-traits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-traits", serde(rename_all = "camelCase"))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct GroupPointer {
    /// Authority that can set the group address
    pub authority: OptionalNonZeroPubkey,
    /// Account address that holds the group
    pub group_address: OptionalNonZeroPubkey,
}

impl Extension for GroupPointer {
    const TYPE: ExtensionType = ExtensionType::GroupPointer;
}
