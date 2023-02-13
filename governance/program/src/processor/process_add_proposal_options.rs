//! Program state processor

use crate::{
    error::GovernanceError,
    state::{
        proposal::{get_proposal_data, OptionVoteResult, ProposalOption},
        token_owner_record::get_token_owner_record_data_for_proposal_owner,
    },
};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    pubkey::Pubkey,
    rent::Rent,
    sysvar::Sysvar,
};
use spl_governance_tools::account::{extend_account_size, AccountMaxSize};

/// Processes AddProposalOption
pub fn process_add_proposal_options(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    options: Vec<String>,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let proposal_info = next_account_info(account_info_iter)?; // 0
    let token_owner_record_info = next_account_info(account_info_iter)?; // 1
    let governance_authority_info = next_account_info(account_info_iter)?; // 2
    let payer_info = next_account_info(account_info_iter)?; // 3
    let system_info = next_account_info(account_info_iter)?; // 4

    let mut proposal_data = get_proposal_data(program_id, proposal_info)?;
    proposal_data.assert_can_add_proposal_options()?;

    let token_owner_record_data = get_token_owner_record_data_for_proposal_owner(
        program_id,
        token_owner_record_info,
        &proposal_data.token_owner_record,
    )?;

    // Proposal owner (TokenOwner) or its governance_delegate must sign this transaction
    token_owner_record_data.assert_token_owner_or_delegate_is_signer(governance_authority_info)?;

    if options.is_empty() {
        return Err(GovernanceError::NoOptionsToAdd.into());
    }
    for option_str in options {
        let po = ProposalOption {
            label: option_str.to_string(),
            vote_weight: 0,
            vote_result: OptionVoteResult::None,
            transactions_executed_count: 0,
            transactions_count: 0,
            transactions_next_index: 0,
        };
        proposal_data.options.push(po);
    }

    let new_account_size = proposal_data
        .get_max_size()
        .ok_or(GovernanceError::CannotCalculateSizeOfProposalData)?;
    let rent = Rent::get()?;
    extend_account_size(
        proposal_info,
        payer_info,
        new_account_size,
        &rent,
        system_info,
    )?;

    proposal_data.serialize(&mut *proposal_info.data.borrow_mut())?;
    Ok(())
}
