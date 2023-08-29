//! Interface error types

use spl_program_error::*;

/// Errors that may be returned by the interface.
/// 
/// Note: Error codes range from 40_000 - (40_000 + n)
#[spl_program_error]
pub enum TokenMetadataError {
    /// Incorrect account provided
    #[error("Incorrect account provided")]
    IncorrectAccount = 40_000, // Error code offset
    /// Mint has no mint authority
    #[error("Mint has no mint authority")]
    MintHasNoMintAuthority,
    /// Incorrect mint authority has signed the instruction
    #[error("Incorrect mint authority has signed the instruction")]
    IncorrectMintAuthority,
    /// Incorrect metadata update authority has signed the instruction
    #[error("Incorrect metadata update authority has signed the instruction")]
    IncorrectUpdateAuthority,
    /// Token metadata has no update authority
    #[error("Token metadata has no update authority")]
    ImmutableMetadata,
    /// Key not found in metadata account
    #[error("Key not found in metadata account")]
    KeyNotFound,
}
