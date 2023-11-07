//! Program state processor

use {
    crate::state::{
        enums::ProposalState, proposal::get_proposal_data,
        token_owner_record::get_token_owner_record_data_for_proposal_owner,
    },
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        clock::Clock,
        entrypoint::ProgramResult,
        pubkey::Pubkey,
        sysvar::Sysvar,
    },
};

/// Processes CompleteProposal instruction
pub fn process_complete_proposal(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let proposal_info = next_account_info(account_info_iter)?; // 0
    let token_owner_record_info = next_account_info(account_info_iter)?; // 1
    let complete_proposal_authority_info = next_account_info(account_info_iter)?; // 2

    let mut proposal_data = get_proposal_data(program_id, proposal_info)?;
    proposal_data.assert_can_complete()?;

    let token_owner_record_data = get_token_owner_record_data_for_proposal_owner(
        program_id,
        token_owner_record_info,
        &proposal_data.token_owner_record,
    )?;
    token_owner_record_data
        .assert_token_owner_or_delegate_is_signer(complete_proposal_authority_info)?;

    let clock = Clock::get()?;
    proposal_data.closed_at = Some(clock.unix_timestamp);
    proposal_data.state = ProposalState::Completed;

    proposal_data.serialize(&mut proposal_info.data.borrow_mut()[..])?;
    Ok(())
}
