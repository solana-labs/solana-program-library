//! Error types

use num_derive::FromPrimitive;
use solana_program::{decode_error::DecodeError, program_error::ProgramError};
use thiserror::Error;

/// Errors that may be returned by the program.
#[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq)]
pub enum RecordError {
    /// Incorrect authority provided on update or delete
    #[error("Incorrect authority provided on update or delete")]
    IncorrectAuthority,

    /// Calculation overflow
    #[error("Calculation overflow")]
    Overflow,
}
impl From<RecordError> for ProgramError {
    fn from(e: RecordError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
impl<T> DecodeError<T> for RecordError {
    fn type_of() -> &'static str {
        "Record Error"
    }
}
