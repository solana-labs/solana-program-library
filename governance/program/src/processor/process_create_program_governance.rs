//! Program state processor

use crate::{
    state::governance::Governance,
    state::{
        enums::GovernanceAccountType,
        governance::{
            assert_valid_create_governance_args, get_program_governance_address_seeds,
            GovernanceConfig,
        },
        realm::get_realm_data,
        token_owner_record::get_token_owner_record_data_for_realm,
    },
    tools::bpf_loader_upgradeable::{
        assert_program_upgrade_authority_is_signer, set_program_upgrade_authority,
    },
};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    pubkey::Pubkey,
    rent::Rent,
    sysvar::Sysvar,
};
use spl_governance_tools::account::create_and_serialize_account_signed;

/// Processes CreateProgramGovernance instruction
pub fn process_create_program_governance(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    config: GovernanceConfig,
    transfer_upgrade_authority: bool,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let realm_info = next_account_info(account_info_iter)?; // 0
    let program_governance_info = next_account_info(account_info_iter)?; // 1

    let governed_program_info = next_account_info(account_info_iter)?; // 2
    let governed_program_data_info = next_account_info(account_info_iter)?; // 3
    let governed_program_upgrade_authority_info = next_account_info(account_info_iter)?; // 4

    let token_owner_record_info = next_account_info(account_info_iter)?; // 5

    let payer_info = next_account_info(account_info_iter)?; // 6
    let bpf_upgrade_loader_info = next_account_info(account_info_iter)?; // 7

    let system_info = next_account_info(account_info_iter)?; // 8

    let rent_sysvar_info = next_account_info(account_info_iter)?; // 9
    let rent = &Rent::from_account_info(rent_sysvar_info)?;

    let governance_authority_info = next_account_info(account_info_iter)?; // 10

    assert_valid_create_governance_args(program_id, &config, realm_info)?;

    let realm_data = get_realm_data(program_id, realm_info)?;
    let token_owner_record_data =
        get_token_owner_record_data_for_realm(program_id, token_owner_record_info, realm_info.key)?;

    token_owner_record_data.assert_token_owner_or_delegate_is_signer(governance_authority_info)?;

    let voter_weight = token_owner_record_data.resolve_voter_weight(
        program_id,
        account_info_iter,
        realm_info.key,
        &realm_data,
    )?;

    token_owner_record_data.assert_can_create_governance(&realm_data, voter_weight)?;

    let program_governance_data = Governance {
        account_type: GovernanceAccountType::ProgramGovernance,
        realm: *realm_info.key,
        governed_account: *governed_program_info.key,
        config,
        proposals_count: 0,
        reserved: [0; 8],
    };

    create_and_serialize_account_signed::<Governance>(
        payer_info,
        program_governance_info,
        &program_governance_data,
        &get_program_governance_address_seeds(realm_info.key, governed_program_info.key),
        program_id,
        system_info,
        rent,
    )?;

    if transfer_upgrade_authority {
        set_program_upgrade_authority(
            governed_program_info.key,
            governed_program_data_info,
            governed_program_upgrade_authority_info,
            program_governance_info,
            bpf_upgrade_loader_info,
        )?;
    } else {
        assert_program_upgrade_authority_is_signer(
            governed_program_info.key,
            governed_program_data_info,
            governed_program_upgrade_authority_info,
        )?;
    }

    Ok(())
}
