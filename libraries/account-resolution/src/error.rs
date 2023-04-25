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

/// Errors that may be returned by the Account Resolution library.
#[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq)]
pub enum AccountResolutionError {
    /// Incorrect account provided
    #[error("Incorrect account provided")]
    IncorrectAccount,
    /// Not enough accounts provided
    #[error("Not enough accounts provided")]
    NotEnoughAccounts,
    /// No value initialized in TLV data
    #[error("No value initialized in TLV data")]
    TlvUninitialized,
    /// Some value initialized in TLV data
    #[error("Some value initialized in TLV data")]
    TlvInitialized,
    /// Provided byte buffer too small for validation pubkeys
    #[error("Provided byte buffer too small for validation pubkeys")]
    BufferTooSmall,
    /// Error in checked math operation
    #[error("Error in checked math operation")]
    CalculationFailure,
    /// Too many pubkeys provided
    #[error("Too many pubkeys provided")]
    TooManyPubkeys,
    /// Provided byte buffer too large for expected type
    #[error("Provided byte buffer too large for expected type")]
    BufferTooLarge,
}
impl From<AccountResolutionError> for ProgramError {
    fn from(e: AccountResolutionError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
impl<T> DecodeError<T> for AccountResolutionError {
    fn type_of() -> &'static str {
        "AccountResolutionError"
    }
}

impl PrintProgramError for AccountResolutionError {
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
            Self::TlvUninitialized => msg!("No value initialized in TLV data"),
            Self::TlvInitialized => msg!("Some value initialized in TLV data"),
            Self::BufferTooSmall => msg!("Provided byte buffer too small for validation pubkeys"),
            Self::CalculationFailure => msg!("Error in checked math operation"),
            Self::TooManyPubkeys => msg!("Too many pubkeys provided"),
            Self::BufferTooLarge => msg!("Provided byte buffer too large for expected type"),
        }
    }
}
