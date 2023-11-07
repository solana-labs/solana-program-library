//! Program state processor

use {
    crate::{
        error::GovernanceError,
        state::{
            enums::GovernanceAccountType,
            governance::get_governance_data,
            required_signatory::{get_required_signatory_address_seeds, RequiredSignatory},
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

/// Processes AddRequiredSignatory instruction
pub fn process_add_required_signatory(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    signatory: Pubkey,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let governance_info = next_account_info(account_info_iter)?; // 0

    let required_signatory_info = next_account_info(account_info_iter)?; // 1

    let payer_info = next_account_info(account_info_iter)?; // 2
    let system_info = next_account_info(account_info_iter)?; // 3

    let rent = Rent::get()?;

    // Only governance PDA via a proposal can authorize change to its own config
    if !governance_info.is_signer {
        return Err(GovernanceError::GovernancePdaMustSign.into());
    };

    let mut governance_data = get_governance_data(program_id, governance_info)?;
    governance_data.required_signatories_count = governance_data
        .required_signatories_count
        .checked_add(1)
        .unwrap();
    governance_data.serialize(&mut governance_info.data.borrow_mut()[..])?;

    let signatory_record_data = RequiredSignatory {
        signatory,
        account_type: GovernanceAccountType::RequiredSignatory,
        governance: *governance_info.key,
        account_version: 0,
    };

    create_and_serialize_account_signed::<RequiredSignatory>(
        payer_info,
        required_signatory_info,
        &signatory_record_data,
        &get_required_signatory_address_seeds(governance_info.key, &signatory),
        program_id,
        system_info,
        &rent,
        0,
    )?;

    Ok(())
}
