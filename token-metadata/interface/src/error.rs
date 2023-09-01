//! Interface error types

use spl_program_error::*;

/// Errors that may be returned by the interface.
#[spl_program_error(hash_error_code_start = 901_952_957)]
pub enum TokenMetadataError {
    /// Incorrect account provided
    #[error("Incorrect account provided")]
    IncorrectAccount,
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
