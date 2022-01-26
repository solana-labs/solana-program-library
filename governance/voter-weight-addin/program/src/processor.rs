//! Program processor

use borsh::BorshDeserialize;
use spl_governance::{
    addins::voter_weight::{VoterWeightAccountType, VoterWeightAction, VoterWeightRecord},
    state::token_owner_record::get_token_owner_record_data_for_realm_and_governing_mint,
};

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Slot,
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
};
use spl_governance_tools::account::create_and_serialize_account;

use crate::instruction::VoterWeightAddinInstruction;

/// Processes an instruction
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    let instruction = VoterWeightAddinInstruction::try_from_slice(input)
        .map_err(|_| ProgramError::InvalidInstructionData)?;

    msg!("GOVERNANCE-VOTER-WEIGHT-INSTRUCTION: {:?}", instruction);

    match instruction {
        VoterWeightAddinInstruction::SetupVoterWeightRecord {
            voter_weight,
            voter_weight_expiry,
            weight_action,
            weight_action_target,
        } => process_setup_voter_weight_record(
            program_id,
            accounts,
            voter_weight,
            voter_weight_expiry,
            weight_action,
            weight_action_target,
        ),
    }
}

/// Processes SetupVoterWeightRecord instruction
pub fn process_setup_voter_weight_record(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    voter_weight: u64,
    voter_weight_expiry: Option<Slot>,
    weight_action: Option<VoterWeightAction>,
    weight_action_target: Option<Pubkey>,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let governance_program_info = next_account_info(account_info_iter)?; // 0
    let realm_info = next_account_info(account_info_iter)?; // 1
    let governing_token_mint_info = next_account_info(account_info_iter)?; // 2
    let token_owner_record_info = next_account_info(account_info_iter)?; // 3
    let voter_weight_record_info = next_account_info(account_info_iter)?; // 4
    let payer_info = next_account_info(account_info_iter)?; // 5
    let system_info = next_account_info(account_info_iter)?; // 6

    let token_owner_record_data = get_token_owner_record_data_for_realm_and_governing_mint(
        governance_program_info.key,
        token_owner_record_info,
        realm_info.key,
        governing_token_mint_info.key,
    )?;

    let voter_weight_record_data = VoterWeightRecord {
        account_type: VoterWeightAccountType::VoterWeightRecord,
        realm: *realm_info.key,
        governing_token_mint: *governing_token_mint_info.key,
        governing_token_owner: token_owner_record_data.governing_token_owner,
        voter_weight,
        voter_weight_expiry,
        weight_action,
        weight_action_target,
    };

    create_and_serialize_account(
        payer_info,
        voter_weight_record_info,
        &voter_weight_record_data,
        program_id,
        system_info,
    )?;

    Ok(())
}
