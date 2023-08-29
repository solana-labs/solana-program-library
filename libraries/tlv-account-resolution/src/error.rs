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
    /// Too many pubkeys provided
    #[error("Too many pubkeys provided")]
    TooManyPubkeys,
    /// Failed to parse `Pubkey` from bytes
    #[error("Failed to parse `Pubkey` from bytes")]
    InvalidPubkey,
    /// Attempted to deserialize an `AccountMeta` but the underlying type has
    /// PDA configs rather than a fixed address
    #[error(
        "Attempted to deserialize an `AccountMeta` but the underlying type has PDA configs rather \
         than a fixed address"
    )]
    AccountTypeNotAccountMeta,
    /// Provided list of seed configurations too large for a validation account
    #[error("Provided list of seed configurations too large for a validation account")]
    SeedConfigsTooLarge,
    /// Not enough bytes available to pack seed configuration
    #[error("Not enough bytes available to pack seed configuration")]
    NotEnoughBytesForSeed,
    /// The provided bytes are not valid for a seed configuration
    #[error("The provided bytes are not valid for a seed configuration")]
    InvalidBytesForSeed,
    /// Tried to pack an invalid seed configuration
    #[error("Tried to pack an invalid seed configuration")]
    InvalidSeedConfig,
    /// Instruction data too small for seed configuration
    #[error("Instruction data too small for seed configuration")]
    InstructionDataTooSmall,
    /// Could not find account at specified index
    #[error("Could not find account at specified index")]
    AccountNotFound,
    /// Error in checked math operation
    #[error("Error in checked math operation")]
    CalculationFailure,
}
impl AccountResolutionError {
    /// Offset to avoid conflict with implementing program error codes
    const PROGRAM_ERROR_OFFSET: u32 = 20000;

    /// Returns the error code
    pub fn error_code(self) -> u32 {
        (self as u32).saturating_add(Self::PROGRAM_ERROR_OFFSET)
    }
}

impl From<AccountResolutionError> for ProgramError {
    fn from(e: AccountResolutionError) -> Self {
        ProgramError::Custom(e.error_code())
    }
}

impl<T> DecodeError<T> for AccountResolutionError {
    fn type_of() -> &'static str {
        "TokenError"
    }
}

impl PrintProgramError for AccountResolutionError {
    fn print<E>(&self)
    where
        E: 'static + std::error::Error + DecodeError<E> + num_traits::FromPrimitive,
    {
        msg!("{}", self);
    }
}
