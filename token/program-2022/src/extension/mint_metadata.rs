use solana_program::pubkey::Pubkey;

use {
    crate::extension::{Extension, ExtensionType},
    bytemuck::{Pod, Zeroable},
};

/// Mint metadata extension data for mints.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct MintMetadata {
    /// Schema of metadata
    pub schema: u8,
    /// Additional accounts required for transfer
    pub address: Pubkey,
}
impl Extension for MintMetadata {
    const TYPE: ExtensionType = ExtensionType::MintMetadata;
}
