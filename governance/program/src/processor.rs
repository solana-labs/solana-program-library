//! Instruction processor

use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, pubkey::Pubkey};

use crate::error::GovernanceError;

/// Processes an instruction
pub fn process_instruction(
    _program_id: &Pubkey,
    _accounts: &[AccountInfo],
    _input: &[u8],
) -> ProgramResult {
    Err(GovernanceError::InvalidInstruction.into())
}
