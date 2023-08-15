//! Interface error types

use spl_program_error::*;

/// Errors that may be returned by the interface.
#[spl_program_error]
pub enum TokenGroupError {
    /// Incorrect account provided
    #[error("Incorrect account provided")]
    IncorrectAccount,
    /// Incorrect authority has signed the instruction
    #[error("Incorrect authority has signed the instruction")]
    IncorrectAuthority,
    /// Size is greater than proposed max size
    #[error("Size is greater than proposed max size")]
    SizeExceedsNewMaxSize,
    /// Size is greater than max size
    #[error("Size is greater than max size")]
    SizeExceedsMaxSize,
    /// Group has no update authority
    #[error("Group has no update authority")]
    ImmutableGroup,
    /// Member has no update authority
    #[error("Member has no update authority")]
    ImmutableMember,
    /// Incorrect group provided
    #[error("Incorrect group provided")]
    IncorrectGroup,
    /// Incorrect member provided
    #[error("Incorrect member provided")]
    IncorrectMember,
    /// Operation overflowed
    #[error("Operation overflowed")]
    Overflow,
}
