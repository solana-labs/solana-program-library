//! Program state processor
use crate::{
    error::GovernanceError,
    state::{
        custom_single_signer_transaction::{CustomSingleSignerTransaction, INSTRUCTION_LIMIT},
        enums::GovernanceAccountType,
        governance::Governance,
        proposal::Proposal,
        proposal_state::{ProposalState, MAX_TRANSACTIONS},
    },
    utils::{
        assert_account_equiv, assert_draft, assert_initialized, assert_is_permissioned,
        assert_token_program_is_correct, assert_uninitialized,
    },
};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    program_pack::Pack,
    pubkey::Pubkey,
};

/// Create a new timelock txn
pub fn process_add_custom_single_signer_transaction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    slot: u64,
    instruction: [u8; INSTRUCTION_LIMIT],
    position: u8,
    instruction_end_index: u16,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let proposal_txn_account_info = next_account_info(account_info_iter)?;
    let proposal_state_account_info = next_account_info(account_info_iter)?;
    let signatory_account_info = next_account_info(account_info_iter)?;
    let signatory_validation_account_info = next_account_info(account_info_iter)?;
    let proposal_account_info = next_account_info(account_info_iter)?;
    let governance_account_info = next_account_info(account_info_iter)?;
    let transfer_authority_info = next_account_info(account_info_iter)?;
    let governance_mint_authority_info = next_account_info(account_info_iter)?;
    let token_program_account_info = next_account_info(account_info_iter)?;

    let mut proposal_state: ProposalState = assert_initialized(proposal_state_account_info)?;
    let proposal: Proposal = assert_initialized(proposal_account_info)?;
    let governance: Governance = assert_initialized(governance_account_info)?;

    let mut proposal_txn: CustomSingleSignerTransaction =
        assert_uninitialized(proposal_txn_account_info)?;

    if position as usize >= MAX_TRANSACTIONS {
        return Err(GovernanceError::TooHighPositionInTxnArrayError.into());
    }

    if instruction_end_index as usize >= INSTRUCTION_LIMIT as usize {
        return Err(GovernanceError::InvalidInstructionEndIndex.into());
    }

    assert_account_equiv(
        signatory_validation_account_info,
        &proposal.signatory_validation,
    )?;
    assert_account_equiv(proposal_state_account_info, &proposal.state)?;
    assert_draft(&proposal_state)?;
    assert_token_program_is_correct(&proposal, token_program_account_info)?;
    assert_is_permissioned(
        program_id,
        signatory_account_info,
        signatory_validation_account_info,
        proposal_account_info,
        token_program_account_info,
        transfer_authority_info,
        governance_mint_authority_info,
    )?;

    if slot < governance.minimum_slot_waiting_period {
        return Err(GovernanceError::MustBeAboveMinimumWaitingPeriod.into());
    };

    proposal_txn.account_type = GovernanceAccountType::CustomSingleSignerTransaction;
    proposal_txn.slot = slot;
    proposal_txn.instruction = instruction;
    proposal_txn.instruction_end_index = instruction_end_index;
    proposal_state.transactions[position as usize] = *proposal_txn_account_info.key;
    proposal_state.number_of_transactions =
        match proposal_state.number_of_transactions.checked_add(1) {
            Some(val) => val,
            None => return Err(GovernanceError::NumericalOverflow.into()),
        };

    ProposalState::pack(
        proposal_state,
        &mut proposal_state_account_info.data.borrow_mut(),
    )?;

    CustomSingleSignerTransaction::pack(
        proposal_txn,
        &mut proposal_txn_account_info.data.borrow_mut(),
    )?;

    Ok(())
}
