//! State transition types
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{entrypoint::ProgramResult, program_error::ProgramError};

/// Uninitialized value of entity
pub const UNINITIALIZED_VALUE: u8 = 0;

/// Account with data
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct DataAccount {
    /// value
    pub value: u8,
}

impl DataAccount {
    /// DataAccount LEN
    pub const LEN: usize = 1;
    /// Check if already initialized
    pub fn uninitialized(&self) -> ProgramResult {
        if self.value == UNINITIALIZED_VALUE {
            Ok(())
        } else {
            Err(ProgramError::AccountAlreadyInitialized)
        }
    }
    /// Error if not initialized
    pub fn initialized(&self) -> ProgramResult {
        if self.value != UNINITIALIZED_VALUE {
            Ok(())
        } else {
            Err(ProgramError::UninitializedAccount)
        }
    }
}