use solana_program::{instruction::InstructionError, program_error::ProgramError};
use solana_sdk::{transaction::TransactionError, transport::TransportError};

pub fn map_transaction_error(transport_error: TransportError) -> ProgramError {
    match transport_error {
        TransportError::TransactionError(TransactionError::InstructionError(
            _,
            InstructionError::Custom(error_index),
        )) => ProgramError::Custom(error_index),
        _ => panic!("TEST-ERROR: {:?}", transport_error),
    }
}
