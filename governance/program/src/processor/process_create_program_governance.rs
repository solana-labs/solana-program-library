//! Program state processor

use crate::{
    state::governance::Governance,
    state::{
        enums::GovernanceAccountType,
        governance::{
            assert_is_valid_governance_config, get_program_governance_address_seeds,
            GovernanceConfig,
        },
    },
    tools::{
        account::create_and_serialize_account_signed,
        bpf_loader_upgradeable::{
            assert_program_upgrade_authority_is_signer, set_program_upgrade_authority,
        },
    },
};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    pubkey::Pubkey,
    rent::Rent,
    sysvar::Sysvar,
};

/// Processes CreateProgramGovernance instruction
pub fn process_create_program_governance(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    config: GovernanceConfig,
    transfer_upgrade_authority: bool,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let realm_info = next_account_info(account_info_iter)?; // 0
    let program_governance_info = next_account_info(account_info_iter)?; // 0

    let governed_program_data_info = next_account_info(account_info_iter)?; // 1
    let governed_program_upgrade_authority_info = next_account_info(account_info_iter)?; // 2

    let payer_info = next_account_info(account_info_iter)?; // 3
    let bpf_upgrade_loader_info = next_account_info(account_info_iter)?; // 4

    let system_info = next_account_info(account_info_iter)?; // 5

    let rent_sysvar_info = next_account_info(account_info_iter)?; // 6
    let rent = &Rent::from_account_info(rent_sysvar_info)?;

    assert_is_valid_governance_config(&config, &realm_info)?;

    let program_governance_data = Governance {
        account_type: GovernanceAccountType::ProgramGovernance,
        config: config.clone(),
        proposals_count: 0,
    };

    create_and_serialize_account_signed::<Governance>(
        payer_info,
        &program_governance_info,
        &program_governance_data,
        &get_program_governance_address_seeds(&config.realm, &config.governed_account),
        program_id,
        system_info,
        rent,
    )?;

    if transfer_upgrade_authority {
        set_program_upgrade_authority(
            &config.governed_account,
            governed_program_data_info,
            governed_program_upgrade_authority_info,
            program_governance_info,
            bpf_upgrade_loader_info,
        )?;
    } else {
        assert_program_upgrade_authority_is_signer(
            &config.governed_account,
            &governed_program_data_info,
            &governed_program_upgrade_authority_info,
        )?;
    }

    Ok(())
}
