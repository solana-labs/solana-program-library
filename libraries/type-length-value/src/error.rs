//! Error types

use spl_program_error::*;

/// Errors that may be returned by the Token program.
#[spl_program_error(hash_error_code_start = 1_202_666_432)]
pub enum TlvError {
    /// Type not found in TLV data
    #[error("Type not found in TLV data")]
    TypeNotFound,
    /// Type already exists in TLV data
    #[error("Type already exists in TLV data")]
    TypeAlreadyExists,
}
