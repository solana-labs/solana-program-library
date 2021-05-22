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
    enums::ProposalState, proposal::deserialize_proposal_raw,
    signatory_record::deserialize_signatory_record,
};

/// Processes SignOffProposal instruction
pub fn process_sign_off_proposal(_program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let proposal_info = next_account_info(account_info_iter)?; // 0

    let signatory_record_info = next_account_info(account_info_iter)?; // 1
    let signatory_info = next_account_info(account_info_iter)?; // 2

    let clock_info = next_account_info(account_info_iter)?; // 3
    let clock = Clock::from_account_info(clock_info)?;

    let mut proposal_data = deserialize_proposal_raw(proposal_info)?;
    proposal_data.assert_can_sign_off()?;

    let mut signatory_record_data =
        deserialize_signatory_record(signatory_record_info, proposal_info.key, signatory_info.key)?;
    signatory_record_data.assert_can_sign_off(signatory_info)?;

    signatory_record_data.signed_off = true;
    signatory_record_data.serialize(&mut *signatory_record_info.data.borrow_mut())?;

    if proposal_data.signatories_signed_off_count == 0 {
        proposal_data.signing_off_at = Some(clock.slot);
        proposal_data.state = ProposalState::SigningOff;
    }

    proposal_data.signatories_signed_off_count = proposal_data
        .signatories_signed_off_count
        .checked_add(1)
        .unwrap();

    // If all Signatories signed off we can start voting
    if proposal_data.signatories_signed_off_count == proposal_data.signatories_count {
        proposal_data.voting_at = Some(clock.slot);
        proposal_data.state = ProposalState::Voting;
    }

    proposal_data.serialize(&mut *proposal_info.data.borrow_mut())?;

    Ok(())
}
