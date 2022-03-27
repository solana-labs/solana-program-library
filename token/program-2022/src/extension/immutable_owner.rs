use {
    crate::extension::{Extension, ExtensionType},
    bytemuck::{Pod, Zeroable},
};

/// Indicates that the Account owner authority cannot be changed
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
#[repr(transparent)]
pub struct ImmutableOwner;

impl Extension for ImmutableOwner {
    const TYPE: ExtensionType = ExtensionType::ImmutableOwner;
}
