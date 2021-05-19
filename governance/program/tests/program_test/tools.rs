use std::{convert::TryFrom, fs::File, io::Read, path::PathBuf};

use solana_program::{instruction::InstructionError, program_error::ProgramError};
use solana_sdk::{transaction::TransactionError, transport::TransportError};

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

fn get_test_program_path(name: &str) -> PathBuf {
    let mut pathbuf = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    pathbuf.push("tests/program_test/programs");
    pathbuf.push(name);
    pathbuf.set_extension("_so");
    pathbuf
}

pub fn read_test_program_elf(name: &str) -> Vec<u8> {
    let path = get_test_program_path(name);
    let mut file = File::open(&path).unwrap_or_else(|err| {
        panic!("Failed to open {}: {}", path.display(), err);
    });
    let mut elf = Vec::new();
    file.read_to_end(&mut elf).unwrap();

    elf
}
