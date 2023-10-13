//! Program error types

use spl_program_error::*;

/// Program specific errors
#[spl_program_error]
pub enum FeatureGateError {
    /// Operation overflowed
    #[error("Operation overflowed")]
    Overflow,
    /// Feature not inactive
    #[error("Feature not inactive")]
    FeatureAlreadyActivated,
}
