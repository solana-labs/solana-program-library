//! Error types
use {
    solana_decode_error::DecodeError,
    solana_msg::msg,
    solana_program_error::{PrintProgramError, ProgramError},
};

/// Errors that may be returned by the Token program.
#[repr(u32)]
#[derive(Clone, Debug, Eq, thiserror::Error, num_derive::FromPrimitive, PartialEq)]
pub enum TlvError {
    /// Type not found in TLV data
    #[error("Type not found in TLV data")]
    TypeNotFound = 1_202_666_432,
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
            TlvError::TypeNotFound => {
                msg!("Type not found in TLV data")
            }
            TlvError::TypeAlreadyExists => {
                msg!("Type already exists in TLV data")
            }
        }
    }
}
