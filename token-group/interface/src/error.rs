//! Interface error types

use spl_program_error::*;

/// Errors that may be returned by the interface.
#[spl_program_error]
pub enum TokenGroupError {
    /// Size is greater than proposed max size
    #[error("Size is greater than proposed max size")]
    SizeExceedsNewMaxSize,
    /// Size is greater than max size
    #[error("Size is greater than max size")]
    SizeExceedsMaxSize,
}
