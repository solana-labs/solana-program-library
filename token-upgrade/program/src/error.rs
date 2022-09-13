//! Error types

use {
    num_derive::FromPrimitive,
    solana_program::{decode_error::DecodeError, program_error::ProgramError},
    thiserror::Error,
};

/// Errors that may be returned by the program.
#[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq)]
pub enum TokenUpgradeError {
    // 0
    /// Account does not match address derivation
    #[error("Account does not match address derivation")]
    InvalidOwner,
    /// Decimals of original and new token mint do not match
    #[error("Decimals of original and new token mint do not match")]
    DecimalsMismatch,
}
impl From<TokenUpgradeError> for ProgramError {
    fn from(e: TokenUpgradeError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
impl<T> DecodeError<T> for TokenUpgradeError {
    fn type_of() -> &'static str {
        "TokenUpgradeError"
    }
}
