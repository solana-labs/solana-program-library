//! Program state processor

use borsh::BorshSerialize;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    entrypoint::ProgramResult,
    pubkey::Pubkey,
    sysvar::Sysvar,
};

use crate::{
    state::{
        enums::ProposalState, proposal::get_proposal_data,
        signatory_record::get_signatory_record_data_for_seeds,
        token_owner_record::get_token_owner_record_data_for_proposal_owner,
    },
    tools::account::dispose_account,
};

/// Processes RemoveSignatory instruction
pub fn process_remove_signatory(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    signatory: Pubkey,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let proposal_info = next_account_info(account_info_iter)?; // 0
    let token_owner_record_info = next_account_info(account_info_iter)?; // 1
    let governance_authority_info = next_account_info(account_info_iter)?; // 2

    let signatory_record_info = next_account_info(account_info_iter)?; // 3
    let beneficiary_info = next_account_info(account_info_iter)?; // 4

    let clock_info = next_account_info(account_info_iter)?; // 5
    let clock = Clock::from_account_info(clock_info)?;

    let mut proposal_data = get_proposal_data(proposal_info)?;
    proposal_data.assert_can_edit_signatories()?;

    let token_owner_record_data = get_token_owner_record_data_for_proposal_owner(
        token_owner_record_info,
        &proposal_data.token_owner_record,
    )?;

    token_owner_record_data.assert_token_owner_or_delegate_is_signer(governance_authority_info)?;

    let signatory_record_data =
        get_signatory_record_data_for_seeds(signatory_record_info, proposal_info.key, &signatory)?;
    signatory_record_data.assert_can_remove_signatory()?;

    proposal_data.signatories_count = proposal_data.signatories_count.checked_sub(1).unwrap();

    // If all the remaining signatories signed already then we can start voting
    if proposal_data.signatories_count > 0
        && proposal_data.signatories_signed_off_count == proposal_data.signatories_count
    {
        proposal_data.voting_at = Some(clock.slot);
        proposal_data.state = ProposalState::Voting;
    }

    proposal_data.serialize(&mut *proposal_info.data.borrow_mut())?;

    dispose_account(signatory_record_info, beneficiary_info);

    Ok(())
}
