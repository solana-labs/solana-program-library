//! Program state processor
use crate::{
    error::TimelockError,
    state::proposal::Proposal,
    state::proposal_state::ProposalState,
    utils::{
        assert_account_equiv, assert_draft, assert_initialized, assert_is_permissioned,
        assert_token_program_is_correct,
    },
};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    program_pack::Pack,
    pubkey::Pubkey,
};

/// Removes a txn from a transaction set
pub fn process_remove_transaction(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let proposal_state_account_info = next_account_info(account_info_iter)?;
    let proposal_txn_account_info = next_account_info(account_info_iter)?;
    let signatory_account_info = next_account_info(account_info_iter)?;
    let signatory_validation_account_info = next_account_info(account_info_iter)?;
    let proposal_account_info = next_account_info(account_info_iter)?;
    let transfer_authority_info = next_account_info(account_info_iter)?;
    let proposal_authority_account_info = next_account_info(account_info_iter)?;
    let token_program_account_info = next_account_info(account_info_iter)?;

    let mut proposal_state: ProposalState = assert_initialized(proposal_state_account_info)?;
    let proposal: Proposal = assert_initialized(proposal_account_info)?;
    assert_token_program_is_correct(&proposal, token_program_account_info)?;
    assert_account_equiv(proposal_state_account_info, &proposal.state)?;
    assert_account_equiv(
        signatory_validation_account_info,
        &proposal.signatory_validation,
    )?;
    assert_draft(&proposal_state)?;
    assert_is_permissioned(
        program_id,
        signatory_account_info,
        signatory_validation_account_info,
        proposal_account_info,
        token_program_account_info,
        transfer_authority_info,
        proposal_authority_account_info,
    )?;

    let mut found: bool = false;
    for n in 0..proposal_state.transactions.len() {
        if proposal_state.transactions[n].to_bytes() == proposal_txn_account_info.key.to_bytes() {
            let zeros: [u8; 32] = [0; 32];
            proposal_state.transactions[n] = Pubkey::new_from_array(zeros);
            found = true;
            break;
        }
    }

    if !found {
        return Err(TimelockError::TimelockTransactionNotFoundError.into());
    }

    proposal_state.number_of_transactions =
        match proposal_state.number_of_transactions.checked_sub(1) {
            Some(val) => val,
            None => return Err(TimelockError::NumericalOverflow.into()),
        };

    ProposalState::pack(
        proposal_state,
        &mut proposal_state_account_info.data.borrow_mut(),
    )?;

    Ok(())
}
