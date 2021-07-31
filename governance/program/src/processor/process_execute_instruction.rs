//! Program state processor

use borsh::BorshSerialize;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    entrypoint::ProgramResult,
    instruction::Instruction,
    program::invoke_signed,
    pubkey::Pubkey,
    sysvar::Sysvar,
};

use crate::state::{
    enums::{InstructionExecutionStatus, ProposalState},
    governance::get_governance_data,
    proposal::get_proposal_data_for_governance,
    proposal_instruction::get_proposal_instruction_data_for_proposal,
};

/// Processes ExecuteInstruction instruction
pub fn process_execute_instruction(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let governance_info = next_account_info(account_info_iter)?; // 0
    let proposal_info = next_account_info(account_info_iter)?; // 1
    let proposal_instruction_info = next_account_info(account_info_iter)?; // 2

    let clock_info = next_account_info(account_info_iter)?; // 3
    let clock = Clock::from_account_info(clock_info)?;

    let governance_data = get_governance_data(program_id, governance_info)?;

    let mut proposal_data =
        get_proposal_data_for_governance(program_id, proposal_info, governance_info.key)?;

    let mut proposal_instruction_data = get_proposal_instruction_data_for_proposal(
        program_id,
        proposal_instruction_info,
        proposal_info.key,
    )?;

    proposal_data
        .assert_can_execute_instruction(&proposal_instruction_data, clock.unix_timestamp)?;

    // Execute instruction with Governance PDA as signer
    let instruction = Instruction::from(&proposal_instruction_data.instruction);

    let instruction_account_infos = account_info_iter.as_slice();

    let mut governance_seeds = governance_data.get_governance_address_seeds()?.to_vec();
    let (_, bump_seed) = Pubkey::find_program_address(&governance_seeds, program_id);
    let bump = &[bump_seed];
    governance_seeds.push(bump);

    invoke_signed(
        &instruction,
        instruction_account_infos,
        &[&governance_seeds[..]],
    )?;

    // Update proposal and instruction accounts
    if proposal_data.state == ProposalState::Succeeded {
        proposal_data.executing_at = Some(clock.unix_timestamp);
        proposal_data.state = ProposalState::Executing;
    }

    proposal_data.instructions_executed_count = proposal_data
        .instructions_executed_count
        .checked_add(1)
        .unwrap();

    // Checking for Executing and ExecutingWithErrors states because instruction can still be executed after being flagged with error
    // The check for instructions_executed_count ensures Proposal can't be transitioned to Completed state from ExecutingWithErrors
    if (proposal_data.state == ProposalState::Executing
        || proposal_data.state == ProposalState::ExecutingWithErrors)
        && proposal_data.instructions_executed_count == proposal_data.instructions_count
    {
        proposal_data.closed_at = Some(clock.unix_timestamp);
        proposal_data.state = ProposalState::Completed;
    }

    proposal_data.serialize(&mut *proposal_info.data.borrow_mut())?;

    proposal_instruction_data.executed_at = Some(clock.unix_timestamp);
    proposal_instruction_data.execution_status = InstructionExecutionStatus::Success;
    proposal_instruction_data.serialize(&mut *proposal_instruction_info.data.borrow_mut())?;

    Ok(())
}
