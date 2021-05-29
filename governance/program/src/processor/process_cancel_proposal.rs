//! Program state processor

use borsh::BorshSerialize;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    pubkey::Pubkey,
};

use crate::state::{
    enums::ProposalState, proposal::get_proposal_data,
    token_owner_record::get_token_owner_record_data_for_proposal_owner,
};

/// Processes CancelProposal instruction
pub fn process_cancel_proposal(_program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let proposal_info = next_account_info(account_info_iter)?; // 0
    let token_owner_record_info = next_account_info(account_info_iter)?; // 1
    let governance_authority_info = next_account_info(account_info_iter)?; // 2

    let mut proposal_data = get_proposal_data(proposal_info)?;
    proposal_data.assert_can_cancel()?;

    let token_owner_record_data = get_token_owner_record_data_for_proposal_owner(
        token_owner_record_info,
        &proposal_data.token_owner_record,
    )?;

    token_owner_record_data.assert_token_owner_or_delegate_is_signer(governance_authority_info)?;

    proposal_data.state = ProposalState::Cancelled;
    proposal_data.serialize(&mut *proposal_info.data.borrow_mut())?;

    Ok(())
}
