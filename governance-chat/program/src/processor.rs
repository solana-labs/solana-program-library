//! Program processor

use crate::instruction::GovernanceChatInstruction;
use borsh::BorshDeserialize;

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
};
use spl_governance::state::proposal::get_proposal_data;

/// Processes an instruction
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    let instruction = GovernanceChatInstruction::try_from_slice(input)
        .map_err(|_| ProgramError::InvalidInstructionData)?;

    msg!("GOVERNANCE-CHAT-INSTRUCTION: {:?}", instruction);

    match instruction {
        GovernanceChatInstruction::PostMessage {} => process_post_message(program_id, accounts),
    }
}

/// Processes PostMessage instruction
pub fn process_post_message(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let proposal_info = next_account_info(account_info_iter)?; // 0
    let mut _proposal_data = get_proposal_data(program_id, proposal_info)?;

    Ok(())
}
