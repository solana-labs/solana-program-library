//! Program state processor

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    entrypoint::ProgramResult,
    pubkey::Pubkey,
    sysvar::Sysvar,
};

use crate::{
    error::GovernanceError,
    state::{enums::ProposalState, proposal::get_proposal_data},
};

/// Processes CompleteProposal instruction
pub fn process_complete_proposal(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let proposal_info = next_account_info(account_info_iter)?; // 0

    let mut proposal_data = get_proposal_data(program_id, proposal_info)?;

    let clock = Clock::get()?;

    // proposal succeeded while there is no transaction at any option -> Completed
    if proposal_data.state == ProposalState::Succeeded
        && proposal_data
            .options
            .iter()
            .all(|o| o.transactions_count == 0)
    {
        proposal_data.closed_at = Some(clock.unix_timestamp);
        proposal_data.state = ProposalState::Completed;
    } else {
        return Err(GovernanceError::InvalidStateForCompleteProposal.into());
    }

    proposal_data.serialize(&mut *proposal_info.data.borrow_mut())?;
    Ok(())
}
