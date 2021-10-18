//! Error types

use num_derive::FromPrimitive;
use solana_program::{
    decode_error::DecodeError,
    msg,
    program_error::{PrintProgramError, ProgramError},
};
use thiserror::Error;

/// Errors that may be returned by the GovernanceTools
#[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq)]
pub enum GovernanceToolsError {
    /// Account already initialized
    #[error("Account already initialized")]
    AccountAlreadyInitialized = 1100,

    /// Account doesn't exist
    #[error("Account doesn't exist")]
    AccountDoesNotExist,

    /// Invalid account owner
    #[error("Invalid account owner")]
    InvalidAccountOwner,

    /// Invalid Account type
    #[error("Invalid Account type")]
    InvalidAccountType,
}

impl PrintProgramError for GovernanceToolsError {
    fn print<E>(&self) {
        msg!("GOVERNANCE-TOOLS-ERROR: {}", &self.to_string());
    }
}

impl From<GovernanceToolsError> for ProgramError {
    fn from(e: GovernanceToolsError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

impl<T> DecodeError<T> for GovernanceToolsError {
    fn type_of() -> &'static str {
        "Governance Tools Error"
    }
}
