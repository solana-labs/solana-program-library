#[cfg(feature = "serde-traits")]
use serde::{Deserialize, Serialize};
use {
    crate::extension::{Extension, ExtensionType},
    bytemuck::{Pod, Zeroable},
    spl_pod::{optional_keys::OptionalNonZeroPubkey, primitives::PodBool},
};

/// Instruction types for the pausable extension
pub mod instruction;
/// Instruction processor for the pausable extension
pub mod processor;

/// Indicates that the tokens from this mint can be paused
#[cfg_attr(feature = "serde-traits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-traits", serde(rename_all = "camelCase"))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
#[repr(C)]
pub struct PausableConfig {
    /// Authority that can pause or resume activity on the mint
    pub authority: OptionalNonZeroPubkey,
    /// Whether minting / transferring / burning tokens is paused
    pub paused: PodBool,
}

/// Indicates that the tokens from this account belong to a pausable mint
#[cfg_attr(feature = "serde-traits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-traits", serde(rename_all = "camelCase"))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
#[repr(transparent)]
pub struct PausableAccount;

impl Extension for PausableConfig {
    const TYPE: ExtensionType = ExtensionType::Pausable;
}

impl Extension for PausableAccount {
    const TYPE: ExtensionType = ExtensionType::PausableAccount;
}
