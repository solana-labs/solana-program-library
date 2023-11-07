#[cfg(feature = "serde-traits")]
use serde::{Deserialize, Serialize};
use {
    crate::extension::{BaseState, BaseStateWithExtensions, Extension, ExtensionType},
    bytemuck::{Pod, Zeroable},
    solana_program::pubkey::Pubkey,
    spl_pod::optional_keys::OptionalNonZeroPubkey,
};

/// Permanent delegate extension data for mints.
#[repr(C)]
#[cfg_attr(feature = "serde-traits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-traits", serde(rename_all = "camelCase"))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct PermanentDelegate {
    /// Optional permanent delegate for transferring or burning tokens
    pub delegate: OptionalNonZeroPubkey,
}
impl Extension for PermanentDelegate {
    const TYPE: ExtensionType = ExtensionType::PermanentDelegate;
}

/// Attempts to get the permanent delegate from the TLV data, returning None
/// if the extension is not found
pub fn get_permanent_delegate<S: BaseState, BSE: BaseStateWithExtensions<S>>(
    state: &BSE,
) -> Option<Pubkey> {
    state
        .get_extension::<PermanentDelegate>()
        .ok()
        .and_then(|e| Option::<Pubkey>::from(e.delegate))
}
