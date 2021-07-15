//! Error types

use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use solana_program::{
    decode_error::DecodeError,
    msg,
    program_error::{PrintProgramError, ProgramError},
};
use thiserror::Error;

/// Errors that may be returned by the Heap program.
#[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq)]
pub enum HeapProgramError {
    /// Wrong node account sent
    #[error("Wrong node account sent")]
    WrongNodeAccount,
    /// Wrong authority was sent
    #[error("Wrong authority was sent")]
    WrongAuthority,
    /// Node data can't be empty
    #[error("Node data can't be empty")]
    InvalidNodesData,
    /// Node indexes are out of range
    #[error("Node indexes are out of range")]
    NodeIndexesOutOfRange,
    /// Calculation error
    #[error("Calculation error")]
    CalculationError,
    /// Node are not related to each others
    #[error("Node are not related to each others")]
    NodesAreNotRelated,
}
impl From<HeapProgramError> for ProgramError {
    fn from(e: HeapProgramError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
impl<T> DecodeError<T> for HeapProgramError {
    fn type_of() -> &'static str {
        "HeapProgramError"
    }
}

impl PrintProgramError for HeapProgramError {
    fn print<E>(&self)
    where
        E: 'static + std::error::Error + DecodeError<E> + PrintProgramError + FromPrimitive,
    {
        match self {
            HeapProgramError::WrongNodeAccount => msg!("Wrong node account sent"),
            HeapProgramError::WrongAuthority => msg!("Wrong authority was sent"),
            HeapProgramError::InvalidNodesData => msg!("Node data can't be empty"),
            HeapProgramError::NodeIndexesOutOfRange => msg!("Node indexes are out of range"),
            HeapProgramError::CalculationError => msg!("Calculation error"),
            HeapProgramError::NodesAreNotRelated => msg!("Node are not related to each others"),
        }
    }
}
