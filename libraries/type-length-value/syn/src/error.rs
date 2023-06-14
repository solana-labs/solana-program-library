//! Error types for the TLV parser

/// Error types for the TLV parser
#[derive(Clone, Debug, Eq, thiserror::Error, PartialEq)]
pub enum SplTlvError {
    /// TLV namespace attribute not provided
    #[error("TLV namespace attribute not provided")]
    TlvNamespaceAttributeNotProvided,
    /// Error parsing TLV namespace attribute
    #[error("Error parsing TLV namespace attribute")]
    TlvNamespaceAttributeParseError,
}
