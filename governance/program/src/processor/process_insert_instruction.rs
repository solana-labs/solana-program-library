//! Program state processor

use std::cmp::Ordering;

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    pubkey::Pubkey,
    rent::Rent,
    sysvar::Sysvar,
};
use spl_governance_tools::account::create_and_serialize_account_signed;

use crate::{
    error::GovernanceError,
    state::{
        enums::{GovernanceAccountType, InstructionExecutionStatus},
        governance::get_governance_data,
        proposal::get_proposal_data_for_governance,
        proposal_instruction::{
            get_proposal_instruction_address_seeds, InstructionData, ProposalInstructionV2,
        },
        token_owner_record::get_token_owner_record_data_for_proposal_owner,
    },
};

/// Processes InsertInstruction instruction
pub fn process_insert_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    option_index: u16,
    instruction_index: u16,
    hold_up_time: u32,
    instruction: InstructionData,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let governance_info = next_account_info(account_info_iter)?; // 0
    let proposal_info = next_account_info(account_info_iter)?; // 1
    let token_owner_record_info = next_account_info(account_info_iter)?; // 2
    let governance_authority_info = next_account_info(account_info_iter)?; // 3

    let proposal_instruction_info = next_account_info(account_info_iter)?; // 4

    let payer_info = next_account_info(account_info_iter)?; // 5
    let system_info = next_account_info(account_info_iter)?; // 6

    let rent_sysvar_info = next_account_info(account_info_iter)?; // 7
    let rent = &Rent::from_account_info(rent_sysvar_info)?;

    if !proposal_instruction_info.data_is_empty() {
        return Err(GovernanceError::InstructionAlreadyExists.into());
    }

    let governance_data = get_governance_data(program_id, governance_info)?;

    if hold_up_time < governance_data.config.min_instruction_hold_up_time {
        return Err(GovernanceError::InstructionHoldUpTimeBelowRequiredMin.into());
    }

    let mut proposal_data =
        get_proposal_data_for_governance(program_id, proposal_info, governance_info.key)?;
    proposal_data.assert_can_edit_instructions()?;

    let token_owner_record_data = get_token_owner_record_data_for_proposal_owner(
        program_id,
        token_owner_record_info,
        &proposal_data.token_owner_record,
    )?;

    token_owner_record_data.assert_token_owner_or_delegate_is_signer(governance_authority_info)?;

    let option = &mut proposal_data.options[option_index as usize];

    match instruction_index.cmp(&option.instructions_next_index) {
        Ordering::Greater => return Err(GovernanceError::InvalidInstructionIndex.into()),
        // If the index is the same as instructions_next_index then we are adding a new instruction
        // If the index is below instructions_next_index then we are inserting into an existing empty space
        Ordering::Equal => {
            option.instructions_next_index = option.instructions_next_index.checked_add(1).unwrap();
        }
        Ordering::Less => {}
    }

    option.instructions_count = option.instructions_count.checked_add(1).unwrap();
    proposal_data.serialize(&mut *proposal_info.data.borrow_mut())?;

    let proposal_instruction_data = ProposalInstructionV2 {
        account_type: GovernanceAccountType::ProposalInstructionV2,
        option_index,
        instruction_index,
        hold_up_time,
        instruction,
        executed_at: None,
        execution_status: InstructionExecutionStatus::None,
        proposal: *proposal_info.key,
    };

    create_and_serialize_account_signed::<ProposalInstructionV2>(
        payer_info,
        proposal_instruction_info,
        &proposal_instruction_data,
        &get_proposal_instruction_address_seeds(
            proposal_info.key,
            &option_index.to_le_bytes(),
            &instruction_index.to_le_bytes(),
        ),
        program_id,
        system_info,
        rent,
    )?;

    Ok(())
}
