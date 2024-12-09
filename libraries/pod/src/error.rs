//! Error types
use {
    solana_decode_error::DecodeError,
    solana_msg::msg,
    solana_program_error::{PrintProgramError, ProgramError},
};

/// Errors that may be returned by the spl-pod library.
#[repr(u32)]
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, num_derive::FromPrimitive)]
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

impl From<PodSliceError> for ProgramError {
    fn from(e: PodSliceError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

impl<T> solana_decode_error::DecodeError<T> for PodSliceError {
    fn type_of() -> &'static str {
        "PodSliceError"
    }
}

impl PrintProgramError for PodSliceError {
    fn print<E>(&self)
    where
        E: 'static
            + std::error::Error
            + DecodeError<E>
            + PrintProgramError
            + num_traits::FromPrimitive,
    {
        match self {
            PodSliceError::CalculationFailure => {
                msg!("Error in checked math operation")
            }
            PodSliceError::BufferTooSmall => {
                msg!("Provided byte buffer too small for expected type")
            }
            PodSliceError::BufferTooLarge => {
                msg!("Provided byte buffer too large for expected type")
            }
        }
    }
}
