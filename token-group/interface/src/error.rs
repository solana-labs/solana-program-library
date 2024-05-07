//! Interface error types

use spl_program_error::*;

/// Errors that may be returned by the interface.
#[spl_program_error(hash_error_code_start = 3_406_457_176)]
pub enum TokenGroupError {
    /// Size is greater than proposed max size
    #[error("Size is greater than proposed max size")]
    SizeExceedsNewMaxSize,
    /// Size is greater than max size
    #[error("Size is greater than max size")]
    SizeExceedsMaxSize,
    /// Group is immutable
    #[error("Group is immutable")]
    ImmutableGroup,
    /// Incorrect mint authority has signed the instruction
    #[error("Incorrect mint authority has signed the instruction")]
    IncorrectMintAuthority,
    /// Incorrect update authority has signed the instruction
    #[error("Incorrect update authority has signed the instruction")]
    IncorrectUpdateAuthority,
    /// Member account should not be the same as the group account
    #[error("Member account should not be the same as the group account")]
    MemberAccountIsGroupAccount,
}
