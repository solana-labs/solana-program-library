//! Interface error types

use spl_program_error::*;

/// Errors that may be returned by the interface.
#[spl_program_error]
pub enum TokenCollectionsError {
    /// Size is greater than proposed max size
    #[error("Size is greater than proposed max size")]
    SizeExceedsNewMaxSize,
    /// Size is greater than max size
    #[error("Size is greater than max size")]
    SizeExceedsMaxSize,
    /// Incorrect mint authority has signed the instruction
    #[error("Incorrect mint authority has signed the instruction")]
    IncorrectMintAuthority,
    /// Incorrect collection update authority has signed the instruction
    #[error("Incorrect collection update authority has signed the instruction")]
    IncorrectUpdateAuthority,
    /// Collection has no update authority
    #[error("Collection has no update authority")]
    ImmutableCollection,
    /// Incorrect collection provided
    #[error("Incorrect collection provided")]
    IncorrectCollection,
}
