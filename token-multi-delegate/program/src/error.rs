//! Error types

use {
    num_derive::FromPrimitive,
    solana_program::{decode_error::DecodeError, program_error::ProgramError},
    thiserror::Error,
};

/// Errors that may be returned by the program.
#[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq)]
pub enum MultiDelegateError {
    #[error("")]
    InsufficientDelegateAmount,
    #[error("")]
    DelegateNotFound,
    #[error("")]
    TokenAccountNotOwnedByProvidedOwner,
}
impl From<MultiDelegateError> for ProgramError {
    fn from(e: MultiDelegateError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
impl<T> DecodeError<T> for MultiDelegateError {
    fn type_of() -> &'static str {
        "MultiDelegateError"
    }
}