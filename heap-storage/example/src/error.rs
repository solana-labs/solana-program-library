//! Error types

use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use solana_program::{
    decode_error::DecodeError,
    msg,
    program_error::{PrintProgramError, ProgramError},
};
use thiserror::Error;

/// Errors that may be returned by the Example program.
#[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq)]
pub enum ExampleProgramError {
    /// Inconsistency between node and node's data account
    #[error("Inconsistency between node and node's data account")]
    WrongNodeDataAcc,

    /// Parent node's value is less then child node's value
    #[error("Parent node's value is less then child node's value")]
    ParentsValueLessThanChild,
}
impl From<ExampleProgramError> for ProgramError {
    fn from(e: ExampleProgramError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
impl<T> DecodeError<T> for ExampleProgramError {
    fn type_of() -> &'static str {
        "ExampleProgramError"
    }
}

impl PrintProgramError for ExampleProgramError {
    fn print<E>(&self)
    where
        E: 'static + std::error::Error + DecodeError<E> + PrintProgramError + FromPrimitive,
    {
        msg!(&self.to_string())
    }
}
