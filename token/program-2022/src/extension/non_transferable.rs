use {
    crate::extension::{Extension, ExtensionType},
    bytemuck::{Pod, Zeroable},
};

#[cfg(feature = "serde-traits")]
use serde::{Deserialize, Serialize};

/// Indicates that the tokens from this mint can't be transfered
#[cfg_attr(feature = "serde-traits", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
#[repr(transparent)]
pub struct NonTransferable;

/// Indicates that the tokens from this account belong to a non-transferable mint
#[cfg_attr(feature = "serde-traits", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
#[repr(transparent)]
pub struct NonTransferableAccount;

impl Extension for NonTransferable {
    const TYPE: ExtensionType = ExtensionType::NonTransferable;
}

impl Extension for NonTransferableAccount {
    const TYPE: ExtensionType = ExtensionType::NonTransferableAccount;
}
