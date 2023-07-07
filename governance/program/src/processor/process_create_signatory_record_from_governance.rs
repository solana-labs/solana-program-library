//! Program state processor

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    pubkey::Pubkey,
    rent::Rent,
    sysvar::Sysvar,
};
use spl_governance_tools::account::create_and_serialize_account_signed;

use crate::{
    error::GovernanceError,
    state::{
        enums::GovernanceAccountType,
        proposal::get_proposal_data_for_governance,
        required_signatory::get_required_signatory_data_for_governance,
        signatory_record::{get_signatory_record_address_seeds, SignatoryRecordV2},
    },
};

/// Processes CreateSignatoryRecordFromGovernance instruction
pub fn process_create_signatory_record_from_governance(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let governance_info = next_account_info(account_info_iter)?; // 0
    let required_signature_info = next_account_info(account_info_iter)?; // 1
    let proposal_info = next_account_info(account_info_iter)?;

    let signatory_record_info = next_account_info(account_info_iter)?; // 3
    let payer_info = next_account_info(account_info_iter)?; // 4
    let system_program = next_account_info(account_info_iter)?; // 5
    let rent = Rent::get()?;

    let _proposal_data =
        get_proposal_data_for_governance(program_id, proposal_info, governance_info.key)?;

    if !signatory_record_info.data_is_empty() {
        return Err(GovernanceError::SignatoryRecordAlreadyExists.into());
    }

    let required_signature_record = get_required_signatory_data_for_governance(
        program_id,
        required_signature_info,
        governance_info.key,
    )?;

    let signatory_record_data = SignatoryRecordV2 {
        account_type: GovernanceAccountType::SignatoryRecordV2,
        proposal: *proposal_info.key,
        signatory: required_signature_record.signatory,
        signed_off: false,
        reserved_v2: [0; 8],
    };

    create_and_serialize_account_signed::<SignatoryRecordV2>(
        payer_info,
        signatory_record_info,
        &signatory_record_data,
        &get_signatory_record_address_seeds(proposal_info.key, &required_signature_record.signatory),
        program_id,
        system_program,
        &rent,
        0,
    )?;

    Ok(())
}
