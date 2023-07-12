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
}
