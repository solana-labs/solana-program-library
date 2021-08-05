//! Program state processor

use crate::{
    state::{
        enums::GovernanceAccountType,
        governance::{
            assert_valid_create_governance_args, get_token_governance_address_seeds, Governance,
            GovernanceConfig,
        },
        realm::get_realm_data,
        token_owner_record::get_token_owner_record_data_for_realm,
    },
    tools::{
        account::create_and_serialize_account_signed,
        spl_token::{assert_spl_token_owner_is_signer, set_spl_token_owner},
    },
};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    pubkey::Pubkey,
    rent::Rent,
    sysvar::Sysvar,
};

/// Processes CreateTokenGovernance instruction
pub fn process_create_token_governance(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    config: GovernanceConfig,
    transfer_token_owner: bool,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let realm_info = next_account_info(account_info_iter)?; // 0
    let token_governance_info = next_account_info(account_info_iter)?; // 1

    let governed_token_info = next_account_info(account_info_iter)?; // 2
    let governed_token_owner_info = next_account_info(account_info_iter)?; // 3

    let token_owner_record_info = next_account_info(account_info_iter)?; // 4

    let payer_info = next_account_info(account_info_iter)?; // 5
    let spl_token_info = next_account_info(account_info_iter)?; // 6

    let system_info = next_account_info(account_info_iter)?; // 7

    let rent_sysvar_info = next_account_info(account_info_iter)?; // 8
    let rent = &Rent::from_account_info(rent_sysvar_info)?;

    assert_valid_create_governance_args(program_id, &config, realm_info)?;

    let realm_data = get_realm_data(program_id, realm_info)?;
    let token_owner_record_data =
        get_token_owner_record_data_for_realm(program_id, token_owner_record_info, realm_info.key)?;

    token_owner_record_data.assert_can_create_governance(&realm_data)?;

    let token_governance_data = Governance {
        account_type: GovernanceAccountType::TokenGovernance,
        realm: *realm_info.key,
        governed_account: *governed_token_info.key,
        config,
        proposals_count: 0,
        reserved: [0; 8],
    };

    create_and_serialize_account_signed::<Governance>(
        payer_info,
        token_governance_info,
        &token_governance_data,
        &get_token_governance_address_seeds(realm_info.key, governed_token_info.key),
        program_id,
        system_info,
        rent,
    )?;

    if transfer_token_owner {
        set_spl_token_owner(
            governed_token_info,
            governed_token_owner_info,
            token_governance_info.key,
            spl_token_info,
        )?;
    } else {
        assert_spl_token_owner_is_signer(governed_token_info, governed_token_owner_info)?;
    }

    Ok(())
}
