//! Error types

use num_derive::FromPrimitive;
use solana_program::{
    decode_error::DecodeError,
    msg,
    program_error::{PrintProgramError, ProgramError},
};
use thiserror::Error;

/// Errors that may be returned by the Governance program
#[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq)]
pub enum GovernanceError {
    /// Invalid instruction passed to program
    #[error("Invalid instruction passed to program")]
    InvalidInstruction,

    /// Realm with the given name and governing mints already exists
    #[error("Realm with the given name and governing mints already exists")]
    RealmAlreadyExists,

    /// Invalid Governing Token Mint
    #[error("Invalid Governing Token Mint")]
    InvalidGoverningTokenMint,

    /// Governing Token Owner must sign transaction
    #[error("Governing Token Owner must sign transaction")]
    GoverningTokenOwnerMustSign,

    /// Governing Token Owner or Vote Authority  must sign transaction
    #[error("Governing Token Owner or Vote Authority  must sign transaction")]
    GoverningTokenOwnerOrVoteAuthrotiyMustSign,

    /// All active votes must be relinquished to withdraw governing tokens
    #[error("All active votes must be relinquished to withdraw governing tokens")]
    CannotWithdrawGoverningTokensWhenActiveVotesExist,

    /// Invalid Voter account address
    #[error("Invalid Voter account address")]
    InvalidVoterAccountAddress,

    /// ---- Account Tools Errors -----

    /// Invalid account owner
    #[error("Invalid account owner")]
    InvalidAccountOwner,

    /// ---- Token Tools Errors -----

    /// Invalid Token account owner
    #[error("Invalid Token account owner")]
    InvalidTokenAccountOwner,
}

impl PrintProgramError for GovernanceError {
    fn print<E>(&self) {
        msg!("GOVERNANCE-ERROR: {}", &self.to_string());
    }
}

impl From<GovernanceError> for ProgramError {
    fn from(e: GovernanceError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

impl<T> DecodeError<T> for GovernanceError {
    fn type_of() -> &'static str {
        "Governance Error"
    }
}
