use {
    solana_program::{instruction::InstructionError, program_error::ProgramError},
    solana_sdk::{signature::Keypair, transaction::TransactionError, transport::TransportError},
    std::convert::TryFrom,
};

/// TODO: Add to Solana SDK
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
        )) => match instruction_error {
            // In solana-sdk v1.19.0, there is a ProgramError for
            // InstructionError::IncorrectAuthority. This results in the error mapping
            // returning two different values: one for sdk < v1.19 and another for sdk >= v1.19.0.
            // To avoid this situation, handle InstructionError::IncorrectAuthority earlier.
            // Can be removed when Solana v1.19.0 becomes a stable channel (also need to update the
            // test assert for
            // `test_create_program_governance_with_incorrect_upgrade_authority_error`)
            InstructionError::IncorrectAuthority => {
                ProgramInstructionError::IncorrectAuthority.into()
            }
            _ => ProgramError::try_from(instruction_error).unwrap_or_else(|ie| match ie {
                InstructionError::IncorrectAuthority => unreachable!(),
                InstructionError::PrivilegeEscalation => {
                    ProgramInstructionError::PrivilegeEscalation.into()
                }
                _ => panic!("TEST-INSTRUCTION-ERROR {:?}", ie),
            }),
        },
        _ => panic!("TEST-TRANSPORT-ERROR: {:?}", transport_error),
    }
}

pub fn clone_keypair(source: &Keypair) -> Keypair {
    Keypair::from_bytes(&source.to_bytes()).unwrap()
}

/// NOP (No Operation) Override function
#[allow(non_snake_case)]
pub fn NopOverride<T>(_: &mut T) {}
