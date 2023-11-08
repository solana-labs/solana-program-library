//! Program state processor

use {
    crate::{
        error::GovernanceError,
        state::{
            enums::GovernanceAccountType,
            governance::get_governance_data,
            proposal::get_proposal_data_for_governance,
            required_signatory::get_required_signatory_data_for_governance,
            signatory_record::{get_signatory_record_address_seeds, SignatoryRecordV2},
            token_owner_record::get_token_owner_record_data_for_proposal_owner,
        },
    },
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint::ProgramResult,
        pubkey::Pubkey,
        rent::Rent,
        sysvar::Sysvar,
    },
    spl_governance_tools::account::create_and_serialize_account_signed,
};

/// Processes AddSignatory instruction
pub fn process_add_signatory(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    signatory: Pubkey,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let governance_info = next_account_info(account_info_iter)?; // 0
    let proposal_info = next_account_info(account_info_iter)?; // 1
    let signatory_record_info = next_account_info(account_info_iter)?; // 2

    let payer_info = next_account_info(account_info_iter)?; // 3
    let system_info = next_account_info(account_info_iter)?; // 4

    let governance_data = get_governance_data(program_id, governance_info)?;

    let mut proposal_data =
        get_proposal_data_for_governance(program_id, proposal_info, governance_info.key)?;
    proposal_data.assert_can_edit_signatories()?;

    if !signatory_record_info.data_is_empty() {
        return Err(GovernanceError::SignatoryRecordAlreadyExists.into());
    }

    // All required signatories must be added before additional signatories can be
    // added
    if proposal_data.signatories_count < governance_data.required_signatories_count {
        let required_signatory_info = next_account_info(account_info_iter)?; // 5
        let required_signatory_data = get_required_signatory_data_for_governance(
            program_id,
            required_signatory_info,
            governance_info.key,
        )?;

        if required_signatory_data.signatory != signatory {
            return Err(GovernanceError::InvalidSignatoryAddress.into());
        }
    } else {
        let token_owner_record_info = next_account_info(account_info_iter)?; // 5
        let governance_authority_info = next_account_info(account_info_iter)?; // 6

        let token_owner_record_data = get_token_owner_record_data_for_proposal_owner(
            program_id,
            token_owner_record_info,
            &proposal_data.token_owner_record,
        )?;

        token_owner_record_data
            .assert_token_owner_or_delegate_is_signer(governance_authority_info)?;
    }

    let rent = Rent::get()?;

    let signatory_record_data = SignatoryRecordV2 {
        account_type: GovernanceAccountType::SignatoryRecordV2,
        proposal: *proposal_info.key,
        signatory,
        signed_off: false,
        reserved_v2: [0; 8],
    };

    create_and_serialize_account_signed::<SignatoryRecordV2>(
        payer_info,
        signatory_record_info,
        &signatory_record_data,
        &get_signatory_record_address_seeds(proposal_info.key, &signatory),
        program_id,
        system_info,
        &rent,
        0,
    )?;

    proposal_data.signatories_count = proposal_data.signatories_count.checked_add(1).unwrap();
    proposal_data.serialize(&mut proposal_info.data.borrow_mut()[..])?;

    Ok(())
}
