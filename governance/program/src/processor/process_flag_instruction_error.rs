//! Program state processor

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    entrypoint::ProgramResult,
    pubkey::Pubkey,
    sysvar::Sysvar,
};

use crate::state::{
    enums::{InstructionExecutionStatus, ProposalState},
    proposal::get_proposal_data,
    proposal_instruction::get_proposal_instruction_data_for_proposal,
    token_owner_record::get_token_owner_record_data_for_proposal_owner,
};

/// Processes FlagInstructionError instruction
pub fn process_flag_instruction_error(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let proposal_info = next_account_info(account_info_iter)?; // 0
    let token_owner_record_info = next_account_info(account_info_iter)?; // 1
    let governance_authority_info = next_account_info(account_info_iter)?; // 2

    let proposal_instruction_info = next_account_info(account_info_iter)?; // 3

    let clock_info = next_account_info(account_info_iter)?; // 4
    let clock = Clock::from_account_info(clock_info)?;

    let mut proposal_data = get_proposal_data(program_id, proposal_info)?;

    let mut proposal_instruction_data = get_proposal_instruction_data_for_proposal(
        program_id,
        proposal_instruction_info,
        proposal_info.key,
    )?;

    proposal_data
        .assert_can_flag_instruction_error(&proposal_instruction_data, clock.unix_timestamp)?;

    let token_owner_record_data = get_token_owner_record_data_for_proposal_owner(
        program_id,
        token_owner_record_info,
        &proposal_data.token_owner_record,
    )?;

    token_owner_record_data.assert_token_owner_or_delegate_is_signer(governance_authority_info)?;

    // If this is the first instruction to be executed then set executing_at timestamp
    // It indicates when we started executing instructions for the Proposal and the fact we only flag it as error is irrelevant here
    if proposal_data.state == ProposalState::Succeeded {
        proposal_data.executing_at = Some(clock.unix_timestamp);
    }

    proposal_data.state = ProposalState::ExecutingWithErrors;
    proposal_data.serialize(&mut *proposal_info.data.borrow_mut())?;

    proposal_instruction_data.execution_status = InstructionExecutionStatus::Error;
    proposal_instruction_data.serialize(&mut *proposal_instruction_info.data.borrow_mut())?;

    Ok(())
}
