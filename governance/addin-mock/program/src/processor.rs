//! Program processor

use borsh::BorshDeserialize;

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Slot,
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
};
use spl_governance_addin_api::{
    max_voter_weight::MaxVoterWeightRecord,
    voter_weight::{VoterWeightAction, VoterWeightRecord},
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
        VoterWeightAddinInstruction::SetupMaxVoterWeightRecord {
            max_voter_weight,
            max_voter_weight_expiry,
        } => process_setup_max_voter_weight_record(
            program_id,
            accounts,
            max_voter_weight,
            max_voter_weight_expiry,
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

    let realm_info = next_account_info(account_info_iter)?; // 0
    let governing_token_mint_info = next_account_info(account_info_iter)?; // 1
    let governing_token_owner_info = next_account_info(account_info_iter)?; // 2
    let voter_weight_record_info = next_account_info(account_info_iter)?; // 3
    let payer_info = next_account_info(account_info_iter)?; // 4
    let system_info = next_account_info(account_info_iter)?; // 5

    let voter_weight_record_data = VoterWeightRecord {
        account_discriminator: VoterWeightRecord::ACCOUNT_DISCRIMINATOR,
        realm: *realm_info.key,
        governing_token_mint: *governing_token_mint_info.key,
        governing_token_owner: *governing_token_owner_info.key,
        voter_weight,
        voter_weight_expiry,
        weight_action,
        weight_action_target,
        reserved: [0; 8],
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

/// Processes SetupMaxVoterWeightRecord instruction
pub fn process_setup_max_voter_weight_record(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    max_voter_weight: u64,
    max_voter_weight_expiry: Option<Slot>,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let realm_info = next_account_info(account_info_iter)?; // 0
    let governing_token_mint_info = next_account_info(account_info_iter)?; // 1
    let max_voter_weight_record_info = next_account_info(account_info_iter)?; // 2
    let payer_info = next_account_info(account_info_iter)?; // 3
    let system_info = next_account_info(account_info_iter)?; // 4

    let max_voter_weight_record_data = MaxVoterWeightRecord {
        account_discriminator: MaxVoterWeightRecord::ACCOUNT_DISCRIMINATOR,
        realm: *realm_info.key,
        governing_token_mint: *governing_token_mint_info.key,
        max_voter_weight,
        max_voter_weight_expiry,
        reserved: [0; 8],
    };

    create_and_serialize_account(
        payer_info,
        max_voter_weight_record_info,
        &max_voter_weight_record_data,
        program_id,
        system_info,
    )?;

    Ok(())
}
