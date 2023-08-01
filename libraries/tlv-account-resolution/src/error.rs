//! Error types

use spl_program_error::*;

/// Errors that may be returned by the Account Resolution library.
#[spl_program_error]
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
    /// Invalid account type
    #[error("Invalid account type")]
    InvalidAccountType,
    /// Attempted to deserialize an `AccountMeta` but the underlying type was
    /// `AccountMetaPda`
    #[error(
        "Attempted to deserialize an `AccountMeta` but the underlying type was `AccountMetaPda`"
    )]
    RequiredAccountNotAccountMeta,
    /// Attempted to deserialize an `AccountMetaPda` but the underlying type was
    /// `AccountMeta`
    #[error(
        "Attempted to deserialize an `AccountMetaPda` but the underlying type was `AccountMeta`"
    )]
    RequiredAccountNotPda,
    /// Provided list of seed configurations too large for a validation account
    #[error("Provided list of seed configurations too large for a validation account")]
    SeedConfigsTooLarge,
    /// Not enough bytes available to pack seed configuration
    #[error("Not enough bytes available to pack seed configuration")]
    NotEnoughBytesForSeed,
    /// The provided bytes are not valid for a seed configuration
    #[error("The provided bytes are not valid for a seed configuration")]
    InvalidBytesForSeed,
    /// Could not find account at specified index
    #[error("Could not find account at specified index")]
    AccountNotFound,
}
