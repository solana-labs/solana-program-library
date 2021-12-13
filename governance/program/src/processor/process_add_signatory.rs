//! Program state processor

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    pubkey::Pubkey,
    rent::Rent,
    sysvar::Sysvar,
};
use spl_governance_tools::account::create_and_serialize_account_signed;

use crate::state::{
    enums::GovernanceAccountType,
    proposal::get_proposal_data,
    signatory_record::{get_signatory_record_address_seeds, SignatoryRecord},
    token_owner_record::get_token_owner_record_data_for_proposal_owner,
};

/// Processes AddSignatory instruction
pub fn process_add_signatory(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    signatory: Pubkey,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let proposal_info = next_account_info(account_info_iter)?; // 0
    let token_owner_record_info = next_account_info(account_info_iter)?; // 1
    let governance_authority_info = next_account_info(account_info_iter)?; // 2

    let signatory_record_info = next_account_info(account_info_iter)?; // 3

    let payer_info = next_account_info(account_info_iter)?; // 4
    let system_info = next_account_info(account_info_iter)?; // 5

    let rent_sysvar_info = next_account_info(account_info_iter)?; // 6
    let rent = &Rent::from_account_info(rent_sysvar_info)?;

    let mut proposal_data = get_proposal_data(program_id, proposal_info)?;
    proposal_data.assert_can_edit_signatories()?;

    let token_owner_record_data = get_token_owner_record_data_for_proposal_owner(
        program_id,
        token_owner_record_info,
        &proposal_data.token_owner_record,
    )?;

    token_owner_record_data.assert_token_owner_or_delegate_is_signer(governance_authority_info)?;

    let signatory_record_data = SignatoryRecord {
        account_type: GovernanceAccountType::SignatoryRecord,
        proposal: *proposal_info.key,
        signatory,
        signed_off: false,
    };

    create_and_serialize_account_signed::<SignatoryRecord>(
        payer_info,
        signatory_record_info,
        &signatory_record_data,
        &get_signatory_record_address_seeds(proposal_info.key, &signatory),
        program_id,
        system_info,
        rent,
    )?;

    proposal_data.signatories_count = proposal_data.signatories_count.checked_add(1).unwrap();
    proposal_data.serialize(&mut *proposal_info.data.borrow_mut())?;

    Ok(())
}
