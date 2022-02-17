use {
    crate::extension::{Extension, ExtensionType},
    bytemuck::{Pod, Zeroable},
};

/// Indicates that the tokens from this mint can't be transfered
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
#[repr(transparent)]
pub struct NonTransferable;

impl Extension for NonTransferable {
    const TYPE: ExtensionType = ExtensionType::NonTransferable;
}
