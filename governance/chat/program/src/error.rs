//! Error types

use num_derive::FromPrimitive;
use solana_program::{
    decode_error::DecodeError,
    msg,
    program_error::{PrintProgramError, ProgramError},
};
use thiserror::Error;

/// Errors that may be returned by the GovernanceChat program
#[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq)]
pub enum GovernanceChatError {
    /// Owner doesn't have enough governing tokens to comment on Proposal
    #[error("Owner doesn't have enough governing tokens to comment on Proposal")]
    NotEnoughTokensToCommentProposal = 900,

    /// Account already initialized
    #[error("Account already initialized")]
    AccountAlreadyInitialized,
}

impl PrintProgramError for GovernanceChatError {
    fn print<E>(&self) {
        msg!("GOVERNANCE-CHAT-ERROR: {}", &self.to_string());
    }
}

impl From<GovernanceChatError> for ProgramError {
    fn from(e: GovernanceChatError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

impl<T> DecodeError<T> for GovernanceChatError {
    fn type_of() -> &'static str {
        "Governance Chat Error"
    }
}
