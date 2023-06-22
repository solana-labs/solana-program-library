//! Error types for the `hash_input` parser

/// Error types for the `hash_input` parser
#[derive(Clone, Debug, Eq, thiserror::Error, PartialEq)]
pub enum SplDiscriminateError {
    /// Discriminator hash_input attribute not provided
    #[error("Discriminator `hash_input` attribute not provided")]
    HashInputAttributeNotProvided,
    /// Error parsing discriminator hash_input attribute
    #[error("Error parsing discriminator `hash_input` attribute")]
    HashInputAttributeParseError,
}
