//! Error types

use spl_program_error::*;

/// Errors that may be returned by the Account Resolution library.
#[spl_program_error(hash_error_code_start = 2_724_315_840)]
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
    /// Could not find account data at specified index
    #[error("Could not find account data at specified index")]
    AccountDataNotFound,
    /// Account data too small for requested seed configuration
    #[error("Account data too small for requested seed configuration")]
    AccountDataTooSmall,
    /// Failed to fetch account
    #[error("Failed to fetch account")]
    AccountFetchFailed,
    /// Not enough bytes available to pack pubkey data configuration.
    #[error("Not enough bytes available to pack pubkey data configuration")]
    NotEnoughBytesForPubkeyData,
    /// The provided bytes are not valid for a pubkey data configuration
    #[error("The provided bytes are not valid for a pubkey data configuration")]
    InvalidBytesForPubkeyData,
    /// Tried to pack an invalid pubkey data configuration
    #[error("Tried to pack an invalid pubkey data configuration")]
    InvalidPubkeyDataConfig,
}
