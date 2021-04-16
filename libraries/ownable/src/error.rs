//! Error types

use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use solana_program::{
    decode_error::DecodeError,
    msg,
    program_error::{PrintProgramError, ProgramError},
};
use thiserror::Error;

// Errors that may be returned by an Ownable program
#[derive(Clone, Debug, Eq, Error, PartialEq, FromPrimitive)]
pub enum OwnableError {
    #[error("Invalid instruction")]
    InvalidInstruction,
    /// The account cannot be initialized because it is already being used.
    #[error("Account is not the program owner")]
    InvalidOwner,
}
impl From<OwnableError> for ProgramError {
    fn from(e: OwnableError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
impl<T> DecodeError<T> for OwnableError {
    fn type_of() -> &'static str {
        "OwnableError"
    }
}
impl PrintProgramError for OwnableError {
    fn print<E>(&self)
    where
        E: 'static + std::error::Error + DecodeError<E> + PrintProgramError + FromPrimitive,
    {
        match self {
            OwnableError::InvalidInstruction => msg!("Error: Invalid OwnableInstruction"),
            OwnableError::InvalidOwner => msg!("Error: Invalid owner"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::OwnableError;
    use solana_program::program_error::{PrintProgramError, ProgramError};

    fn return_as_program_error() -> ProgramError {
        OwnableError::InvalidOwner.into()
    }

    #[test]
    fn test_print() {
        let error = return_as_program_error();
        error.print::<OwnableError>();
    }

    #[test]
    #[should_panic(expected = "Custom(1)")]
    fn test_unwrap() {
        Err::<(), ProgramError>(return_as_program_error()).unwrap();
    }
}
