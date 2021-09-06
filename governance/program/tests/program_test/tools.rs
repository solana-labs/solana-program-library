use std::convert::TryFrom;

use solana_program::{instruction::InstructionError, program_error::ProgramError};
use solana_sdk::{signature::Keypair, transaction::TransactionError, transport::TransportError};

/// TODO: Add to SDK
/// Instruction errors not mapped in the sdk
pub enum ProgramInstructionError {
    /// Incorrect authority provided
    IncorrectAuthority = 600,

    /// Cross-program invocation with unauthorized signer or writable account
    PrivilegeEscalation,
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
            InstructionError::PrivilegeEscalation => {
                ProgramInstructionError::PrivilegeEscalation.into()
            }
            _ => panic!("TEST-INSTRUCTION-ERROR {:?}", ie),
        }),

        _ => panic!("TEST-TRANSPORT-ERROR: {:?}", transport_error),
    }
}

pub fn clone_keypair(source: &Keypair) -> Keypair {
    Keypair::from_bytes(&source.to_bytes()).unwrap()
}

/// NOP (No Operation) Override function
#[allow(non_snake_case)]
pub fn NopOverride<T>(_: &mut T) {}
