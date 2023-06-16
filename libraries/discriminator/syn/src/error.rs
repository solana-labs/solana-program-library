//! Error types for the namespace parser

/// Error types for the namespace parser
#[derive(Clone, Debug, Eq, thiserror::Error, PartialEq)]
pub enum SplDiscriminatorError {
    /// Discriminator namespace attribute not provided
    #[error("Discriminator namespace attribute not provided")]
    NamespaceAttributeNotProvided,
    /// Error parsing discriminator namespace attribute
    #[error("Error parsing discriminator namespace attribute")]
    NamespaceAttributeParseError,
}
