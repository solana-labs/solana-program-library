//! Error types

use num_derive::FromPrimitive;
use solana_program::{decode_error::DecodeError, program_error::ProgramError};
use thiserror::Error;

/// Errors that may be returned by the Identity program.
#[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq)]
pub enum IdentityError {
    /// Lamport balance below rent-exempt threshold.
    #[error("Lamport balance below rent-exempt threshold")]
    NotRentExempt,
    /// Insufficient funds for the operation requested.
    #[error("Insufficient funds")]
    InsufficientFunds,
    /// Owner does not match.
    #[error("Owner does not match")]
    OwnerMismatch,
    /// The account cannot be initialized because it is already being used.
    #[error("Already in use")]
    AlreadyInUse,
    /// Invalid instruction
    #[error("Invalid instruction")]
    InvalidInstruction,
    /// The provided Identity does not have an attestation registered by the required IDV
    #[error("The provided identity does have an attestation registered by the required IDV")]
    UnauthorizedIdentity,
}
impl From<IdentityError> for ProgramError {
    fn from(e: IdentityError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
impl<T> DecodeError<T> for IdentityError {
    fn type_of() -> &'static str {
        "IdentityError"
    }
}
