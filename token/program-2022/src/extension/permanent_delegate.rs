use {
    crate::{
        extension::{Extension, ExtensionType},
        pod::*,
    },
    bytemuck::{Pod, Zeroable},
};

/// Permanent delegate extension data for mints.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct PermanentDelegate {
    /// Optional permanent delegate for transferring or burning tokens
    pub delegate: OptionalNonZeroPubkey,
}
impl Extension for PermanentDelegate {
    const TYPE: ExtensionType = ExtensionType::PermanentDelegate;
}
