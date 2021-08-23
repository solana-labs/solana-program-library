//! Program processor

use crate::{
    error::GovernanceChatError, instruction::GovernanceChatInstruction, state::Message,
    tools::account::create_and_serialize_account,
};
use borsh::BorshDeserialize;

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvar::Sysvar,
};
use spl_governance::state::{
    governance::get_governance_data, proposal::get_proposal_data_for_governance,
    token_owner_record::get_token_owner_record_data_for_realm,
};

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
        GovernanceChatInstruction::PostMessage { body } => {
            process_post_message(program_id, accounts, body)
        }
    }
}

/// Processes PostMessage instruction
pub fn process_post_message(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    body: String,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let governance_program_info = next_account_info(account_info_iter)?; // 0
    let governance_info = next_account_info(account_info_iter)?; // 1
    let proposal_info = next_account_info(account_info_iter)?; // 1
    let token_owner_record_info = next_account_info(account_info_iter)?; // 1
    let governance_authority_info = next_account_info(account_info_iter)?; // 1

    let message_info = next_account_info(account_info_iter)?; // 1
    let payer_info = next_account_info(account_info_iter)?; // 1
    let system_info = next_account_info(account_info_iter)?; // 6

    let governance_data = get_governance_data(governance_program_info.key, governance_info)?;

    let token_owner_record_data = get_token_owner_record_data_for_realm(
        governance_program_info.key,
        &token_owner_record_info,
        &governance_data.realm,
    )?;

    token_owner_record_data.assert_token_owner_or_delegate_is_signer(governance_authority_info)?;

    // deserialize proposal to assert it belongs to the given governance and hence same realm as the token owner
    let _proposal_data = get_proposal_data_for_governance(
        governance_program_info.key,
        &proposal_info,
        governance_info.key,
    )?;

    // The owner needs to have at least 1 governing token to comment on proposals
    // Note: It can be either community or council token and is irrelevant to the proposal's governing token
    // Note: 1 is currently hardcoded but if different level is required then it should be added to realm config
    if token_owner_record_data.governing_token_deposit_amount < 1 {
        return Err(GovernanceChatError::NotEnoughTokensToCommentProposal.into());
    }

    let clock = Clock::get()?;

    let message_data = Message {
        proposal: *proposal_info.key,
        author: token_owner_record_data.governing_token_owner,
        post_at: clock.unix_timestamp,
        parent: None,
        body,
    };

    create_and_serialize_account(
        payer_info,
        message_info,
        &message_data,
        program_id,
        system_info,
    )?;

    Ok(())
}
