//! Program state processor

use crate::{
    state::{enums::ProposalStateStatus, proposal::Proposal, proposal_state::ProposalState},
    utils::{
        assert_account_equiv, assert_initialized, assert_is_permissioned,
        assert_not_in_voting_or_executing, assert_token_program_is_correct,
    },
};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    program_pack::Pack,
    pubkey::Pubkey,
};

/// Delete timelock set
pub fn process_delete_proposal(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let timelock_state_account_info = next_account_info(account_info_iter)?;
    let admin_account_info = next_account_info(account_info_iter)?;
    let admin_validation_account_info = next_account_info(account_info_iter)?;
    let timelock_set_account_info = next_account_info(account_info_iter)?;
    let transfer_authority_info = next_account_info(account_info_iter)?;
    let timelock_authority_info = next_account_info(account_info_iter)?;
    let token_program_info = next_account_info(account_info_iter)?;

    let mut timelock_state: ProposalState = assert_initialized(timelock_state_account_info)?;
    let timelock_set: Proposal = assert_initialized(timelock_set_account_info)?;

    assert_account_equiv(
        admin_validation_account_info,
        &timelock_set.admin_validation,
    )?;
    assert_account_equiv(timelock_state_account_info, &timelock_set.state)?;
    assert_token_program_is_correct(&timelock_set, token_program_info)?;
    assert_not_in_voting_or_executing(&timelock_state)?;
    assert_is_permissioned(
        program_id,
        admin_account_info,
        admin_validation_account_info,
        timelock_set_account_info,
        token_program_info,
        transfer_authority_info,
        timelock_authority_info,
    )?;
    timelock_state.status = ProposalStateStatus::Deleted;
    ProposalState::pack(
        timelock_state,
        &mut timelock_state_account_info.data.borrow_mut(),
    )?;
    Ok(())
}
