//! Program error types

use spl_program_error::*;

/// Program specific errors
#[spl_program_error]
pub enum FeatureGateError {
    /// Operation overflowed
    #[error("Operation overflowed")]
    Overflow,
    /// Feature already activated
    #[error("Feature already activated")]
    FeatureAlreadyActivated,
    /// Incorrect feature ID
    #[error("Incorrect feature ID")]
    IncorrectFeatureId,
    /// Invalid feature account
    #[error("Invalid feature account")]
    InvalidFeatureAccount,
    /// Missing nonce for authority
    #[error("Missing nonce for authority")]
    MissingNonce,
}
