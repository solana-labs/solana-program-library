use {
    crate::extension::{Extension, ExtensionType, UnsizedExtension},
    spl_token_metadata_interface::state::TokenMetadata,
};

/// Instruction processor for the TokenMetadata extension
pub mod processor;

impl Extension for TokenMetadata {
    const TYPE: ExtensionType = ExtensionType::TokenMetadata;
}
impl UnsizedExtension for TokenMetadata {}
