//! Program state processor

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    pubkey::Pubkey,
};
use spl_governance_tools::account::dispose_account;

use crate::state::{
    proposal::get_proposal_data, signatory_record::get_signatory_record_data_for_seeds,
    token_owner_record::get_token_owner_record_data_for_proposal_owner,
};

/// Processes RemoveSignatory instruction
pub fn process_remove_signatory(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    signatory: Pubkey,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let proposal_info = next_account_info(account_info_iter)?; // 0
    let token_owner_record_info = next_account_info(account_info_iter)?; // 1
    let governance_authority_info = next_account_info(account_info_iter)?; // 2

    let signatory_record_info = next_account_info(account_info_iter)?; // 3
    let beneficiary_info = next_account_info(account_info_iter)?; // 4

    let mut proposal_data = get_proposal_data(program_id, proposal_info)?;
    proposal_data.assert_can_edit_signatories()?;

    let token_owner_record_data = get_token_owner_record_data_for_proposal_owner(
        program_id,
        token_owner_record_info,
        &proposal_data.token_owner_record,
    )?;

    token_owner_record_data.assert_token_owner_or_delegate_is_signer(governance_authority_info)?;

    let signatory_record_data = get_signatory_record_data_for_seeds(
        program_id,
        signatory_record_info,
        proposal_info.key,
        &signatory,
    )?;
    signatory_record_data.assert_can_remove_signatory()?;

    proposal_data.signatories_count = proposal_data.signatories_count.checked_sub(1).unwrap();

    proposal_data.serialize(&mut *proposal_info.data.borrow_mut())?;

    dispose_account(signatory_record_info, beneficiary_info);

    Ok(())
}
