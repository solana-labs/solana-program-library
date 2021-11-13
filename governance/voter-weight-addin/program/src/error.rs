//! Error types

use num_derive::FromPrimitive;
use solana_program::{
    decode_error::DecodeError,
    msg,
    program_error::{PrintProgramError, ProgramError},
};
use thiserror::Error;

/// Errors that may be returned by the VoterWeightAddin program
#[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq)]
pub enum VoterWeightAddinError {
    /// Invalid instruction passed to program
    #[error("Invalid instruction passed to program")]
    InvalidInstruction = 500, // Start Governance custom errors from 500 to avoid conflicts with programs invoked via CPI
    /// Realm with the given name and governing mints already exists
    #[error("Can't create vote_weight_addin with no realm authority")]
    CantAddVoterWeight,
}

impl PrintProgramError for VoterWeightAddinError {
    fn print<E>(&self) {
        msg!("VOTER-WEIGHT-ADDIN-ERROR: {}", &self.to_string());
    }
}

impl From<VoterWeightAddinError> for ProgramError {
    fn from(e: VoterWeightAddinError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

impl<T> DecodeError<T> for VoterWeightAddinError {
    fn type_of() -> &'static str {
        "Voter Weight Addin Error"
    }
}
