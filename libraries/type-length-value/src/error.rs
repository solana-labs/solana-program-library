//! Error types

use {
    num_derive::FromPrimitive,
    solana_program::{
        decode_error::DecodeError,
        msg,
        program_error::{PrintProgramError, ProgramError},
    },
    thiserror::Error,
};

/// Errors that may be returned by the Token program.
#[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq)]
pub enum TlvError {
    /// Type not found in TLV data
    #[error("Type not found in TLV data")]
    TypeNotFound,
    /// Type already exists in TLV data
    #[error("Type already exists in TLV data")]
    TypeAlreadyExists,
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
impl TlvError {
    /// Offset to avoid conflict with implementing program error codes
    const PROGRAM_ERROR_OFFSET: u32 = 10000;

    /// Returns the error code
    pub fn error_code(self) -> u32 {
        (self as u32).saturating_add(Self::PROGRAM_ERROR_OFFSET)
    }
}

impl From<TlvError> for ProgramError {
    fn from(e: TlvError) -> Self {
        ProgramError::Custom(e.error_code())
    }
}

impl<T> DecodeError<T> for TlvError {
    fn type_of() -> &'static str {
        "TokenError"
    }
}

impl PrintProgramError for TlvError {
    fn print<E>(&self)
    where
        E: 'static + std::error::Error + DecodeError<E> + num_traits::FromPrimitive,
    {
        msg!("{}", self);
    }
}
