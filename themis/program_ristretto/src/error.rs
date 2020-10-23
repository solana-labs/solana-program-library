//! Error types

use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use solana_program::program_error::PrintProgramError;
use solana_program::{decode_error::DecodeError, program_error::ProgramError};
use thiserror::Error;

/// Errors that may be returned by the Themis program.
#[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq)]
pub enum ThemisError {
    /// Invalid instruction
    #[error("Invalid instruction")]
    InvalidInstruction,

    /// Account already in use
    #[error("Account in use")]
    AccountInUse,
}
impl From<ThemisError> for ProgramError {
    fn from(e: ThemisError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
impl<T> DecodeError<T> for ThemisError {
    fn type_of() -> &'static str {
        "ThemisError"
    }
}

impl PrintProgramError for ThemisError {
    fn print<E>(&self)
    where
        E: 'static + std::error::Error + DecodeError<E> + PrintProgramError + FromPrimitive,
    {
        match self {
            ThemisError::InvalidInstruction => println!("Error: Invalid instruction"),
            ThemisError::AccountInUse => println!("Error: Account in use"),
        }
    }
}
