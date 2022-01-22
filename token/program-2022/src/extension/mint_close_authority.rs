use {
    crate::{
        extension::{Extension, ExtensionType},
        pod::*,
    },
    bytemuck::{Pod, Zeroable},
};

/// Close authority extension data for mints.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct MintCloseAuthority {
    /// Optional authority to close the mint
    pub close_authority: OptionalNonZeroPubkey,
}
impl Extension for MintCloseAuthority {
    const TYPE: ExtensionType = ExtensionType::MintCloseAuthority;
}
