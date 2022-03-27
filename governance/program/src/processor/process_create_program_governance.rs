//! Program state processor

use crate::{
    state::governance::GovernanceV2,
    state::{
        enums::GovernanceAccountType,
        governance::{
            assert_valid_create_governance_args, get_program_governance_address_seeds,
            GovernanceConfig,
        },
        realm::get_realm_data,
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

    let rent = Rent::get()?;

    let create_authority_info = next_account_info(account_info_iter)?; // 9

    assert_valid_create_governance_args(program_id, &config, realm_info)?;

    let realm_data = get_realm_data(program_id, realm_info)?;

    realm_data.assert_create_authority_can_create_governance(
        program_id,
        realm_info.key,
        token_owner_record_info,
        create_authority_info,
        account_info_iter, // realm_config_info 10, voter_weight_record_info 11
    )?;

    let program_governance_data = GovernanceV2 {
        account_type: GovernanceAccountType::ProgramGovernanceV2,
        realm: *realm_info.key,
        governed_account: *governed_program_info.key,
        config,
        proposals_count: 0,
        reserved: [0; 6],
        voting_proposal_count: 0,
        reserved_v2: [0; 128],
    };

    create_and_serialize_account_signed::<GovernanceV2>(
        payer_info,
        program_governance_info,
        &program_governance_data,
        &get_program_governance_address_seeds(realm_info.key, governed_program_info.key),
        program_id,
        system_info,
        &rent,
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
