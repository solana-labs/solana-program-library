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
    // 0
    /// Type not found in TLV data
    #[error("Type not found in TLV data")]
    TypeNotFound,
    /// Type already exists in TLV data
    #[error("Type already exists in TLV data")]
    TypeAlreadyExists,
}
impl From<TlvError> for ProgramError {
    fn from(e: TlvError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
impl<T> DecodeError<T> for TlvError {
    fn type_of() -> &'static str {
        "TlvError"
    }
}
impl PrintProgramError for TlvError {
    fn print<E>(&self)
    where
        E: 'static
            + std::error::Error
            + DecodeError<E>
            + PrintProgramError
            + num_traits::FromPrimitive,
    {
        match self {
            Self::TypeNotFound => msg!("Type not found in TLV data"),
            Self::TypeAlreadyExists => msg!("Type already exists in TLV data"),
        }
    }
}
