//! Error types

use {
    num_derive::FromPrimitive,
    solana_program::{decode_error::DecodeError, program_error::ProgramError},
    thiserror::Error,
};

/// Errors that may be returned by the program.
#[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq)]
pub enum SlashingError {
    /// Incorrect authority provided on write or close
    #[error("Incorrect authority provided on write or close")]
    IncorrectAuthority,

    /// Invalid proof type
    #[error("Invalid proof type")]
    InvalidProofType,

    /// Calculation overflow
    #[error("Calculation overflow")]
    Overflow,
}

impl From<SlashingError> for ProgramError {
    fn from(e: SlashingError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

impl<T> DecodeError<T> for SlashingError {
    fn type_of() -> &'static str {
        "Slashing Error"
    }
}
