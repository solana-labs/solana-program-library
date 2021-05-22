//! Program state processor

use crate::{
    state::{
        enums::GovernanceAccountType,
        governance::{
            assert_is_valid_governance_config, get_account_governance_address_seeds, Governance,
            GovernanceConfig,
        },
    },
    tools::account::create_and_serialize_account_signed,
};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    pubkey::Pubkey,
    rent::Rent,
    sysvar::Sysvar,
};

/// Processes CreateAccountGovernance instruction
pub fn process_create_account_governance(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    config: GovernanceConfig,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let realm_info = next_account_info(account_info_iter)?; // 0
    let account_governance_info = next_account_info(account_info_iter)?; // 0
    let payer_info = next_account_info(account_info_iter)?; // 1
    let system_info = next_account_info(account_info_iter)?; // 2

    let rent_sysvar_info = next_account_info(account_info_iter)?; // 3
    let rent = &Rent::from_account_info(rent_sysvar_info)?;

    assert_is_valid_governance_config(&config, realm_info)?;

    let account_governance_data = Governance {
        account_type: GovernanceAccountType::AccountGovernance,
        config: config.clone(),
        proposal_count: 0,
    };

    create_and_serialize_account_signed::<Governance>(
        payer_info,
        &account_governance_info,
        &account_governance_data,
        &get_account_governance_address_seeds(&config.realm, &config.governed_account),
        program_id,
        system_info,
        rent,
    )?;

    Ok(())
}
