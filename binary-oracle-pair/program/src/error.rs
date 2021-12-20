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
    /// Amount should be more than zero
    #[error("Amount should be more than zero")]
    InvalidAmount,
    /// Wrong decider account
    #[error("Wrong decider account was sent")]
    WrongDeciderAccount,
    /// Signature missing in transaction
    #[error("Signature missing in transaction")]
    SignatureMissing,
    /// Decision was already made for this pool
    #[error("Decision was already made for this pool")]
    DecisionAlreadyMade,
    /// Decision can't be made in current slot
    #[error("Decision can't be made in current slot")]
    InvalidSlotForDecision,
    /// Deposit can't be made in current slot
    #[error("Deposit can't be made in current slot")]
    InvalidSlotForDeposit,
    /// No decision has been made yet
    #[error("No decision has been made yet")]
    NoDecisionMadeYet,
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
            PoolError::AlreadyInUse => msg!("Error: Pool account already in use"),
            PoolError::DepositAccountInUse => msg!("Error: Deposit account already in use"),
            PoolError::TokenMintInUse => msg!("Error: Token account already in use"),
            PoolError::InvalidAuthorityData => {
                msg!("Error: Failed to generate program account because of invalid data")
            }
            PoolError::InvalidAuthorityAccount => msg!("Error: Invalid authority account provided"),
            PoolError::NotRentExempt => msg!("Error: Lamport balance below rent-exempt threshold"),
            PoolError::InvalidTokenMint => msg!("Error: Input token mint account is not valid"),
            PoolError::InvalidAmount => msg!("Error: Amount should be more than zero"),
            PoolError::WrongDeciderAccount => msg!("Error: Wrong decider account was sent"),
            PoolError::SignatureMissing => msg!("Error: Signature missing in transaction"),
            PoolError::DecisionAlreadyMade => {
                msg!("Error: Decision was already made for this pool")
            }
            PoolError::InvalidSlotForDecision => {
                msg!("Error: Decision can't be made in current slot")
            }
            PoolError::InvalidSlotForDeposit => msg!("Deposit can't be made in current slot"),
            PoolError::NoDecisionMadeYet => msg!("Error: No decision has been made yet"),
        }
    }
}
