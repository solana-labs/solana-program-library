//! Error types

use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use solana_program::program_error::PrintProgramError;
use solana_program::{decode_error::DecodeError, program_error::ProgramError};
use thiserror::Error;

/// Errors that may be returned by the ZOL program.
#[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq)]
pub enum ZolError {
    /// Invalid instruction
    #[error("Invalid instruction")]
    InvalidInstruction,

    /// Unexpected State type
    #[error("Unexpected State type")]
    UnexpectedStateType,

    /// Deposit from invalid account
    #[error("Deposit from invalid account")]
    DepositFromInvalidAccount,

    /// Solvency Proof Verification Failed
    #[error("Solvency proof verification failed")]
    SolvencyProofVerificationFailed,
}
impl From<ZolError> for ProgramError {
    fn from(e: ZolError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
impl<T> DecodeError<T> for ZolError {
    fn type_of() -> &'static str {
        "ZolError"
    }
}

impl PrintProgramError for ZolError {
    fn print<E>(&self)
    where
        E: 'static + std::error::Error + DecodeError<E> + PrintProgramError + FromPrimitive,
    {
        match self {
            ZolError::InvalidInstruction => println!("Error: Invalid instruction"),
            ZolError::UnexpectedStateType => println!("Error: Unexpected State type"),
            ZolError::DepositFromInvalidAccount => println!("Error: Deposit from invalid account"),
            ZolError::SolvencyProofVerificationFailed => {
                println!("Error: Solvency proof verification failed")
            }
        }
    }
}
