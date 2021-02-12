//! Error types

use num_derive::FromPrimitive;
use solana_program::{decode_error::DecodeError, program_error::ProgramError};
use thiserror::Error;

/// Errors that may be returned by the TokenSwap program.
#[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq)]
pub enum CrudError {
    /// Incorrect owner provided on update or delete
    #[error("Incorrect owner provided on update or delete")]
    IncorrectOwner,

    /// Calculation overflow
    #[error("Calculation overflow")]
    Overflow,
}
impl From<CrudError> for ProgramError {
    fn from(e: CrudError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
impl<T> DecodeError<T> for CrudError {
    fn type_of() -> &'static str {
        "CRUD Error"
    }
}
