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
pub enum PermissionedTransferError {
    // 0
    /// Incorrect account provided
    #[error("Incorrect account provided")]
    IncorrectAccount,
    /// Not enough accounts provided
    #[error("Not enough accounts provided")]
    NotEnoughAccounts,
    /// Type not found in TLV data
    #[error("Type not found in TLV data")]
    TypeNotFound,
    /// Type already exists in TLV data
    #[error("Type already exists in TLV data")]
    TypeAlreadyExists,
}
impl From<PermissionedTransferError> for ProgramError {
    fn from(e: PermissionedTransferError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
impl<T> DecodeError<T> for PermissionedTransferError {
    fn type_of() -> &'static str {
        "PermissionedTrasferError"
    }
}

impl PrintProgramError for PermissionedTransferError {
    fn print<E>(&self)
    where
        E: 'static
            + std::error::Error
            + DecodeError<E>
            + PrintProgramError
            + num_traits::FromPrimitive,
    {
        match self {
            Self::IncorrectAccount => msg!("Incorrect account provided"),
            Self::NotEnoughAccounts => msg!("Not enough accounts provided"),
            Self::TypeNotFound => msg!("Type not found in TLV data"),
            Self::TypeAlreadyExists => msg!("Type already exists in TLV data"),
        }
    }
}
