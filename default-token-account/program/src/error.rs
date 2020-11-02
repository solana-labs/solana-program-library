//! Error types

use num_traits::FromPrimitive;
use solana_program::{
    decode_error::DecodeError,
    instruction::InstructionError,
    program_error::{PrintProgramError, ProgramError},
};
use thiserror::Error;

/// Errors that may be returned by the DefaultTokenAccount program.
#[derive(Debug, Error, num_derive::FromPrimitive)]
pub enum DefaultTokenAccountError {
    /// Default token account address is incorrect
    #[error("Default token account address is incorrect")]
    InvalidDefaultTokenAccountAddress,

    /// Token owner does not match
    #[error("Token owner does not match")]
    TokenOwnerMismatch,
}

impl From<DefaultTokenAccountError> for ProgramError {
    fn from(e: DefaultTokenAccountError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

impl From<DefaultTokenAccountError> for InstructionError {
    fn from(e: DefaultTokenAccountError) -> Self {
        InstructionError::Custom(e as u32)
    }
}

impl<T> DecodeError<T> for DefaultTokenAccountError {
    fn type_of() -> &'static str {
        "DefaultTokenAccount Error"
    }
}

impl PrintProgramError for DefaultTokenAccountError {
    fn print<E>(&self)
    where
        E: 'static + std::error::Error + DecodeError<E> + PrintProgramError + FromPrimitive,
    {
        match self {
            DefaultTokenAccountError::InvalidDefaultTokenAccountAddress => {
                println!("Error: Default token account address is incorrect")
            }
            DefaultTokenAccountError::TokenOwnerMismatch => {
                println!("Error: Token owner does not match")
            }
        }
    }
}
