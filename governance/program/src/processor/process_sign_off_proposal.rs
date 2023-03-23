//! Program state processor

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    entrypoint::ProgramResult,
    pubkey::Pubkey,
    sysvar::Sysvar,
};

use crate::state::{
    enums::ProposalState, governance::get_governance_data_for_realm,
    proposal::get_proposal_data_for_governance, realm::assert_is_valid_realm,
    signatory_record::get_signatory_record_data_for_seeds,
    token_owner_record::get_token_owner_record_data_for_proposal_owner,
};

/// Processes SignOffProposal instruction
pub fn process_sign_off_proposal(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let realm_info = next_account_info(account_info_iter)?; // 0
    let governance_info = next_account_info(account_info_iter)?; // 1
    let proposal_info = next_account_info(account_info_iter)?; // 2

    let signatory_info = next_account_info(account_info_iter)?; // 3

    let clock = Clock::get()?;

    assert_is_valid_realm(program_id, realm_info)?;

    // Governance account data is no longer used in the current version but we still have to load it to validate Realm -> Governance -> Proposal relationship
    // It could be replaced with PDA check but the account is going to be needed in future versions once we support mandatory signatories
    // and hence keeping it as it is
    let _governance_data =
        get_governance_data_for_realm(program_id, governance_info, realm_info.key)?;

    let mut proposal_data =
        get_proposal_data_for_governance(program_id, proposal_info, governance_info.key)?;

    proposal_data.assert_can_sign_off()?;

    // If the owner of the proposal hasn't appointed any signatories then can sign off the proposal themself
    if proposal_data.signatories_count == 0 {
        let proposal_owner_record_info = next_account_info(account_info_iter)?; // 4

        let proposal_owner_record_data = get_token_owner_record_data_for_proposal_owner(
            program_id,
            proposal_owner_record_info,
            &proposal_data.token_owner_record,
        )?;

        // Proposal owner (TokenOwner) or its governance_delegate must be the signatory and sign this transaction
        proposal_owner_record_data.assert_token_owner_or_delegate_is_signer(signatory_info)?;

        proposal_data.signing_off_at = Some(clock.unix_timestamp);
    } else {
        let signatory_record_info = next_account_info(account_info_iter)?; // 4

        let mut signatory_record_data = get_signatory_record_data_for_seeds(
            program_id,
            signatory_record_info,
            proposal_info.key,
            signatory_info.key,
        )?;

        signatory_record_data.assert_can_sign_off(signatory_info)?;

        signatory_record_data.signed_off = true;
        signatory_record_data.serialize(&mut *signatory_record_info.data.borrow_mut())?;

        if proposal_data.signatories_signed_off_count == 0 {
            proposal_data.signing_off_at = Some(clock.unix_timestamp);
            proposal_data.state = ProposalState::SigningOff;
        }

        proposal_data.signatories_signed_off_count = proposal_data
            .signatories_signed_off_count
            .checked_add(1)
            .unwrap();
    }

    // If all Signatories signed off we can start voting
    if proposal_data.signatories_signed_off_count == proposal_data.signatories_count {
        proposal_data.voting_at = Some(clock.unix_timestamp);
        proposal_data.voting_at_slot = Some(clock.slot);
        proposal_data.state = ProposalState::Voting;
    }

    proposal_data.serialize(&mut *proposal_info.data.borrow_mut())?;

    Ok(())
}
