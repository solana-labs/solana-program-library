use crate::error::MemoError;
use solana_sdk::{entrypoint::ProgramResult, info, program_error::ProgramError};
use std::{mem::size_of, str::from_utf8};

/// Instructions supported by the memo program
#[repr(C)]
#[derive(Clone, Debug, PartialEq)]
pub enum Instruction<'a> {
    /// Validate UTF-8 encoded characters
    Utf8(&'a str),
}

impl<'a> Instruction<'a> {
    pub fn deserialize(input: &'a [u8]) -> Result<Self, ProgramError> {
        if input.len() < size_of::<u8>() {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(match input[0] {
            0 => {
                info!("Instruction: Utf8");
                let memo = from_utf8(&input[1..])
                    .map_err(|_| ProgramError::from(MemoError::InvalidUtf8))?;
                Self::Utf8(memo)
            }
            _ => return Err(ProgramError::InvalidAccountData),
        })
    }

    pub fn serialize(self: &Self, output: &mut [u8]) -> ProgramResult {
        if output.len() < size_of::<u8>() {
            return Err(ProgramError::InvalidAccountData);
        }
        match self {
            Self::Utf8(memo) => {
                let bytes = memo.as_bytes();
                if output.len() < size_of::<u8>() + bytes.len() {
                    return Err(ProgramError::InvalidAccountData);
                }
                output[0] = 0;
                output[1..].clone_from_slice(&bytes);
            }
        }
        Ok(())
    }
}

// Pulls in the stubs required for `info!()`
#[cfg(not(target_arch = "bpf"))]
solana_sdk_bpf_test::stubs!();
