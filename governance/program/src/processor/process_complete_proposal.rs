//! Program state processor

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    entrypoint::ProgramResult,
    pubkey::Pubkey,
    sysvar::Sysvar,
};

use crate::state::{enums::ProposalState, proposal::get_proposal_data_for_governance};

// TODO: the proposal completition rules needs to be reviewed, see https://github.com/solana-labs/solana-program-library/issues/3772

// TODO: how to parametrize the one year constant
const TIME_FOR_COMPLETE_PROPOSAL_WITH_ERRORS_SECONDS: i64 = 60 * 60 * 24 * 365;

/// Processes CompleteProposal instruction
pub fn process_complete_proposal(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let governance_info = next_account_info(account_info_iter)?; // 0
    let proposal_info = next_account_info(account_info_iter)?; // 1

    let mut proposal_data =
        get_proposal_data_for_governance(program_id, proposal_info, governance_info.key)?;

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
    }

    // proposal execution still finishing with errors and time elapsed -> Completed
    if proposal_data.state == ProposalState::ExecutingWithErrors
        && proposal_data.voting_completed_at.is_some()
        && proposal_data.voting_completed_at.unwrap()
            + TIME_FOR_COMPLETE_PROPOSAL_WITH_ERRORS_SECONDS
            < clock.unix_timestamp
    {
        proposal_data.closed_at = Some(clock.unix_timestamp);
        proposal_data.state = ProposalState::Completed;
    }

    proposal_data.serialize(&mut *proposal_info.data.borrow_mut())?;
    Ok(())
}
