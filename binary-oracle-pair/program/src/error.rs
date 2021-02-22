//! Error types

use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use solana_program::{
    decode_error::DecodeError, msg, program_error::PrintProgramError, program_error::ProgramError,
};
use thiserror::Error;

/// Errors that may be returned by the Binary Oracle Pair program.
#[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq)]
pub enum PoolError {
    /// Invalid instruction data passed in.
    #[error("Failed to unpack instruction data")]
    InstructionUnpackError,
    /// Pool account already in use
    #[error("Pool account already in use")]
    AlreadyInUse,
    /// Deposit account already in use
    #[error("Deposit account already in use")]
    DepositAccountInUse,
    /// Token mint account already in use
    #[error("Token account already in use")]
    TokenMintInUse,
    /// Invalid seed or bump_seed was provided
    #[error("Failed to generate program account because of invalid data")]
    InvalidAuthorityData,
    /// Invalid authority account provided
    #[error("Invalid authority account provided")]
    InvalidAuthorityAccount,
    /// Lamport balance below rent-exempt threshold.
    #[error("Lamport balance below rent-exempt threshold")]
    NotRentExempt,
    /// Expected an SPL Token mint
    #[error("Input token mint account is not valid")]
    InvalidTokenMint,
}

impl From<PoolError> for ProgramError {
    fn from(e: PoolError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

impl<T> DecodeError<T> for PoolError {
    fn type_of() -> &'static str {
        "Binary Oracle Pair Error"
    }
}

impl PrintProgramError for PoolError {
    fn print<E>(&self)
    where
        E: 'static + std::error::Error + DecodeError<E> + PrintProgramError + FromPrimitive,
    {
        match self {
            PoolError::InstructionUnpackError => msg!("Error: Failed to unpack instruction data"),
            PoolError::AlreadyInUse => msg!("Error: Pool account already in use"),
            PoolError::DepositAccountInUse => msg!("Error: Deposit account already in use"),
            PoolError::TokenMintInUse => msg!("Error: Token account already in use"),
            PoolError::InvalidAuthorityData => {
                msg!("Error: Failed to generate program account because of invalid data")
            }
            PoolError::InvalidAuthorityAccount => msg!("Error: Invalid authority account provided"),
            PoolError::NotRentExempt => msg!("Error: Lamport balance below rent-exempt threshold"),
            PoolError::InvalidTokenMint => msg!("Error: Input token mint account is not valid"),
        }
    }
}
