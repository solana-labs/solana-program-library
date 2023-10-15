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
}
