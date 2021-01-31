//! Error types

use num_derive::FromPrimitive;
use solana_program::{decode_error::DecodeError, program_error::ProgramError};
use thiserror::Error;

/// Errors that may be returned by the TokenLending program.
#[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq)]
pub enum TimelockError {
    /// Invalid instruction data passed in.
    #[error("Failed to unpack instruction data")]
    InstructionUnpackError,
    /// The account cannot be initialized because it is already in use.
    #[error("Account is already initialized")]
    AlreadyInitialized,

    /// The account is uninitialized
    #[error("The account is uninitialized when it should have already been initialized")]
    Uninitialized,

    /// Lamport balance below rent-exempt threshold.
    #[error("Lamport balance below rent-exempt threshold")]
    NotRentExempt,

    /// Expected a different SPL Token program
    #[error("Input token program account is not valid")]
    InvalidTokenProgram,

    /// Expected an SPL Token mint
    #[error("Input token mint account is not valid")]
    InvalidTokenMint,

    /// Token initialize mint failed
    #[error("Token initialize mint failed")]
    TokenInitializeMintFailed,
    /// Token initialize account failed
    #[error("Token initialize account failed")]
    TokenInitializeAccountFailed,
    /// Token transfer failed
    #[error("Token transfer failed")]
    TokenTransferFailed,
    /// Token mint to failed
    #[error("Token mint to failed")]
    TokenMintToFailed,
    /// Token burn failed
    #[error("Token burn failed")]
    TokenBurnFailed,
}

impl From<TimelockError> for ProgramError {
    fn from(e: TimelockError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

impl<T> DecodeError<T> for TimelockError {
    fn type_of() -> &'static str {
        "Timelock Error"
    }
}
