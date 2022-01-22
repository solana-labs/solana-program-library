//! Program processor

use crate::{
    error::GovernanceChatError,
    instruction::GovernanceChatInstruction,
    state::{assert_is_valid_chat_message, ChatMessage, GovernanceChatAccountType, MessageBody},
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
    governance::get_governance_data_for_realm, proposal::get_proposal_data_for_governance,
    realm::get_realm_data, token_owner_record::get_token_owner_record_data_for_realm,
};
use spl_governance_tools::account::create_and_serialize_account;

/// Processes an instruction
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    msg!("VERSION:{:?}", env!("CARGO_PKG_VERSION"));

    let instruction = GovernanceChatInstruction::try_from_slice(input)
        .map_err(|_| ProgramError::InvalidInstructionData)?;

    match instruction {
        GovernanceChatInstruction::PostMessage { body, is_reply } => {
            msg!("GOVERNANCE-CHAT-INSTRUCTION: PostMessage");
            process_post_message(program_id, accounts, body, is_reply)
        }
    }
}

/// Processes PostMessage instruction
pub fn process_post_message(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    body: MessageBody,
    is_reply: bool,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let governance_program_info = next_account_info(account_info_iter)?; // 0
    let realm_info = next_account_info(account_info_iter)?; // 1
    let governance_info = next_account_info(account_info_iter)?; // 2
    let proposal_info = next_account_info(account_info_iter)?; // 3
    let token_owner_record_info = next_account_info(account_info_iter)?; // 4
    let governance_authority_info = next_account_info(account_info_iter)?; // 5

    let chat_message_info = next_account_info(account_info_iter)?; // 6

    let payer_info = next_account_info(account_info_iter)?; // 7
    let system_info = next_account_info(account_info_iter)?; // 8

    let reply_to_address = if is_reply {
        let reply_to_info = next_account_info(account_info_iter)?; // 9
        assert_is_valid_chat_message(program_id, reply_to_info)?;
        Some(*reply_to_info.key)
    } else {
        None
    };

    let governance_program_id = governance_program_info.key;
    let realm_data = get_realm_data(governance_program_id, realm_info)?;

    let governance_data =
        get_governance_data_for_realm(governance_program_id, governance_info, realm_info.key)?;

    let token_owner_record_data = get_token_owner_record_data_for_realm(
        governance_program_id,
        token_owner_record_info,
        &governance_data.realm,
    )?;

    token_owner_record_data.assert_token_owner_or_delegate_is_signer(governance_authority_info)?;

    // deserialize proposal to assert it belongs to the given governance and hence belongs to the same realm as the token owner
    let _proposal_data = get_proposal_data_for_governance(
        governance_program_id,
        proposal_info,
        governance_info.key,
    )?;

    let voter_weight = token_owner_record_data.resolve_voter_weight(
        governance_program_id,
        account_info_iter, // 10
        realm_info.key,
        &realm_data,
    )?;

    // The owner needs to have at least voter weight of 1 to comment on proposals
    // Note: It can be either community or council token and is irrelevant to the proposal's governing token
    // Note: 1 is currently hardcoded but if different level is required then it should be added to realm config
    if voter_weight < 1 {
        return Err(GovernanceChatError::NotEnoughTokensToCommentProposal.into());
    }

    let clock = Clock::get()?;

    let chat_message_data = ChatMessage {
        account_type: GovernanceChatAccountType::ChatMessage,
        proposal: *proposal_info.key,
        author: token_owner_record_data.governing_token_owner,
        posted_at: clock.unix_timestamp,
        reply_to: reply_to_address,
        body,
    };

    create_and_serialize_account(
        payer_info,
        chat_message_info,
        &chat_message_data,
        program_id,
        system_info,
    )?;

    Ok(())
}
