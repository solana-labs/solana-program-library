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
    /// Provided list of seed configurations too large for expected type
    #[error("Provided list of seed configurations too large for expected type")]
    SeedConfigsTooLarge,
    /// The byte value provided does not resolve to a valid seed configuration
    #[error("The byte value provided does not resolve to a valid seed configuration")]
    InvalidByteValueForSeed,
    /// Attempted to deserialize an `AccountMeta` but the underlying type was `AccountMetaPda`
    #[error(
        "Attempted to deserialize an `AccountMeta` but the underlying type was `AccountMetaPda`"
    )]
    RequiredAccountNotAccountMeta,
    /// Attempted to deserialize an `AccountMetaPda` but the underlying type was `AccountMeta`
    #[error(
        "Attempted to deserialize an `AccountMetaPda` but the underlying type was `AccountMeta`"
    )]
    RequiredAccountNotPda,
    /// No seeds were provided but one or more PDAs are required by the program
    #[error("No seeds were provided but one or more PDAs are required by the program")]
    SeedsRequired,
    /// Not enough seeds arguments were provided for all PDAs required by the program
    #[error("Not enough seeds arguments were provided for all PDAs required by the program")]
    NotEnoughSeedsProvided,
    /// The provided seeds do not match the required seeds stated by the validation account
    #[error("The provided seeds do not match the required seeds stated by the validation account")]
    SeedsMismatch,
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
            Self::SeedConfigsTooLarge => {
                msg!("Provided list of seed configurations too large for expected type")
            }
            Self::InvalidByteValueForSeed => {
                msg!("The byte value provided does not resolve to a valid seed configuration")
            }
            Self::RequiredAccountNotAccountMeta => {
                msg!("Attempted to deserialize an `AccountMeta` but the underlying type was `AccountMetaPda`")
            }
            Self::RequiredAccountNotPda => msg!("Attempted to deserialize an `AccountMetaPda` but the underlying type was `AccountMeta`"),
            Self::SeedsRequired => msg!("No seeds were provided but one or more PDAs are required by the program"),
            Self::NotEnoughSeedsProvided => msg!("Not enough seeds arguments were provided for all PDAs required by the program"),
            Self::SeedsMismatch => msg!("The provided seeds do not match the required seeds stated by the validation account"),
        }
    }
}
