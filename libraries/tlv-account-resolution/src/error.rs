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
    /// Provided byte buffer too small for validation pubkeys
    #[error("Provided byte buffer too small for validation pubkeys")]
    BufferTooSmall,
    /// Provided byte buffer too large for expected type
    #[error("Provided byte buffer too large for expected type")]
    BufferTooLarge,
    /// Failed to parse `Pubkey` from bytes
    #[error("Failed to parse `Pubkey` from bytes")]
    InvalidPubkey,
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
