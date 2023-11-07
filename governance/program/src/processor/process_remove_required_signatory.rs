use {
    crate::{
        error::GovernanceError,
        state::{
            governance::get_governance_data,
            required_signatory::get_required_signatory_data_for_governance,
        },
    },
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint::ProgramResult,
        pubkey::Pubkey,
    },
    spl_governance_tools::account::dispose_account,
};

pub fn process_remove_required_signatory(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let governance_info = next_account_info(account_info_iter)?; // 0
    let required_signatory_info = next_account_info(account_info_iter)?; // 1
    let beneficiary_info = next_account_info(account_info_iter)?; // 2

    if !governance_info.is_signer {
        return Err(GovernanceError::GovernancePdaMustSign.into());
    };

    let mut governance_data = get_governance_data(program_id, governance_info)?;

    get_required_signatory_data_for_governance(
        program_id,
        required_signatory_info,
        governance_info.key,
    )?;

    governance_data.required_signatories_count = governance_data
        .required_signatories_count
        .checked_sub(1)
        .unwrap();
    governance_data.serialize(&mut governance_info.data.borrow_mut()[..])?;

    dispose_account(required_signatory_info, beneficiary_info)?;

    Ok(())
}
