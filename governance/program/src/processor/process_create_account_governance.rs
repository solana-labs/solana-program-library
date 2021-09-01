//! Program state processor

use crate::{
    state::{
        enums::GovernanceAccountType,
        governance::{
            assert_valid_create_governance_args, get_account_governance_address_seeds, Governance,
            GovernanceConfig,
        },
        realm::get_realm_data,
        token_owner_record::get_token_owner_record_data_for_realm,
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
    let account_governance_info = next_account_info(account_info_iter)?; // 1
    let governed_account_info = next_account_info(account_info_iter)?; // 2

    let token_owner_record_info = next_account_info(account_info_iter)?; // 3

    let payer_info = next_account_info(account_info_iter)?; // 4
    let system_info = next_account_info(account_info_iter)?; // 5

    let rent_sysvar_info = next_account_info(account_info_iter)?; // 6
    let rent = &Rent::from_account_info(rent_sysvar_info)?;

    assert_valid_create_governance_args(program_id, &config, realm_info)?;

    let realm_data = get_realm_data(program_id, realm_info)?;
    let token_owner_record_data =
        get_token_owner_record_data_for_realm(program_id, token_owner_record_info, realm_info.key)?;

    token_owner_record_data.assert_can_create_governance(&realm_data)?;

    let account_governance_data = Governance {
        account_type: GovernanceAccountType::AccountGovernance,
        realm: *realm_info.key,
        governed_account: *governed_account_info.key,
        config,
        proposals_count: 0,
        reserved: [0; 8],
    };

    create_and_serialize_account_signed::<Governance>(
        payer_info,
        account_governance_info,
        &account_governance_data,
        &get_account_governance_address_seeds(realm_info.key, governed_account_info.key),
        program_id,
        system_info,
        rent,
    )?;

    Ok(())
}
