//! Program state processor

use borsh::BorshSerialize;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    entrypoint::ProgramResult,
    pubkey::Pubkey,
    sysvar::Sysvar,
};

use crate::state::{
    enums::ProposalState, proposal::get_proposal_data,
    token_owner_record::get_token_owner_record_data_for_proposal_owner,
};

/// Processes CancelProposal instruction
pub fn process_cancel_proposal(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let proposal_info = next_account_info(account_info_iter)?; // 0
    let proposal_owner_record_info = next_account_info(account_info_iter)?; // 1
    let governance_authority_info = next_account_info(account_info_iter)?; // 2

    let clock_info = next_account_info(account_info_iter)?; // 3
    let clock = Clock::from_account_info(clock_info)?;

    let mut proposal_data = get_proposal_data(program_id, proposal_info)?;
    proposal_data.assert_can_cancel()?;

    let mut proposal_owner_record_data = get_token_owner_record_data_for_proposal_owner(
        program_id,
        proposal_owner_record_info,
        &proposal_data.token_owner_record,
    )?;

    proposal_owner_record_data
        .assert_token_owner_or_delegate_is_signer(governance_authority_info)?;

    proposal_owner_record_data.decrease_outstanding_proposal_count();
    proposal_owner_record_data.serialize(&mut *proposal_owner_record_info.data.borrow_mut())?;

    proposal_data.state = ProposalState::Cancelled;
    proposal_data.closed_at = Some(clock.unix_timestamp);

    proposal_data.serialize(&mut *proposal_info.data.borrow_mut())?;

    Ok(())
}
