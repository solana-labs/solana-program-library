use std::convert::TryFrom;

use solana_program::{instruction::InstructionError, program_error::ProgramError};
use solana_sdk::{transaction::TransactionError, transport::TransportError};

/// TODO: Add to SDK
/// Instruction errors not mapped in the sdk
pub enum ProgramInstructionError {
    /// Incorrect authority provided
    IncorrectAuthority,
}

impl From<ProgramInstructionError> for ProgramError {
    fn from(e: ProgramInstructionError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

pub fn map_transaction_error(transport_error: TransportError) -> ProgramError {
    match transport_error {
        TransportError::TransactionError(TransactionError::InstructionError(
            _,
            InstructionError::Custom(error_index),
        )) => ProgramError::Custom(error_index),
        TransportError::TransactionError(TransactionError::InstructionError(
            _,
            instruction_error,
        )) => ProgramError::try_from(instruction_error).unwrap_or_else(|ie| match ie {
            InstructionError::IncorrectAuthority => {
                ProgramInstructionError::IncorrectAuthority.into()
            }
            _ => panic!("TEST-INSTRUCTION-ERROR {:?}", ie),
        }),

        _ => panic!("TEST-TRANSPORT-ERROR: {:?}", transport_error),
    }
}
