//! Program state processor

use crate::{
    state::{
        enums::GovernanceAccountType,
        governance::{
            assert_is_valid_governance_config, get_token_governance_address_seeds, Governance,
            GovernanceConfig,
        },
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

    let payer_info = next_account_info(account_info_iter)?; // 4
    let spl_token_info = next_account_info(account_info_iter)?; // 5

    let system_info = next_account_info(account_info_iter)?; // 6

    let rent_sysvar_info = next_account_info(account_info_iter)?; // 7
    let rent = &Rent::from_account_info(rent_sysvar_info)?;

    assert_is_valid_governance_config(program_id, &config, realm_info)?;

    let token_governance_data = Governance {
        account_type: GovernanceAccountType::TokenGovernance,
        config: config.clone(),
        proposals_count: 0,
    };

    create_and_serialize_account_signed::<Governance>(
        payer_info,
        token_governance_info,
        &token_governance_data,
        &get_token_governance_address_seeds(&config.realm, &config.governed_account),
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
