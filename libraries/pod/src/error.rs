//! Error types
use spl_program_error::*;

/// Errors that may be returned by the spl-pod library.
#[spl_program_error]
pub enum PodSliceError {
    /// Error in checked math operation
    #[error("Error in checked math operation")]
    CalculationFailure,
    /// Provided byte buffer too small for expected type
    #[error("Provided byte buffer too small for expected type")]
    BufferTooSmall,
    /// Provided byte buffer too large for expected type
    #[error("Provided byte buffer too large for expected type")]
    BufferTooLarge,
}
