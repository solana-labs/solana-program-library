//! Program state processor

use crate::{
    state::{
        enums::GovernanceAccountType,
        governance::{
            assert_valid_create_governance_args, get_token_governance_address_seeds,
            GovernanceConfig, GovernanceV2,
        },
        realm::get_realm_data,
    },
    tools::spl_token::{assert_spl_token_owner_is_signer, set_spl_token_account_authority},
};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    program_pack::Pack,
    pubkey::Pubkey,
    rent::Rent,
    sysvar::Sysvar,
};

use spl_governance_tools::account::create_and_serialize_account_signed;
use spl_token::{instruction::AuthorityType, state::Account};

/// Processes CreateTokenGovernance instruction
pub fn process_create_token_governance(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    config: GovernanceConfig,
    transfer_account_authorities: bool,
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

    let rent = Rent::get()?;

    let create_authority_info = next_account_info(account_info_iter)?; // 8

    assert_valid_create_governance_args(program_id, &config, realm_info)?;

    let realm_data = get_realm_data(program_id, realm_info)?;

    realm_data.assert_create_authority_can_create_governance(
        program_id,
        realm_info.key,
        token_owner_record_info,
        create_authority_info,
        account_info_iter, // realm_config_info 9, voter_weight_record_info 10
    )?;

    let token_governance_data = GovernanceV2 {
        account_type: GovernanceAccountType::TokenGovernanceV2,
        realm: *realm_info.key,
        governed_account: *governed_token_info.key,
        config,
        proposals_count: 0,
        reserved: [0; 6],
        voting_proposal_count: 0,
        reserved_v2: [0; 128],
    };

    create_and_serialize_account_signed::<GovernanceV2>(
        payer_info,
        token_governance_info,
        &token_governance_data,
        &get_token_governance_address_seeds(realm_info.key, governed_token_info.key),
        program_id,
        system_info,
        &rent,
    )?;

    if transfer_account_authorities {
        set_spl_token_account_authority(
            governed_token_info,
            governed_token_owner_info,
            token_governance_info.key,
            AuthorityType::AccountOwner,
            spl_token_info,
        )?;

        // If the token account has close_authority then transfer it as well
        let token_account_data = Account::unpack(&governed_token_info.data.borrow())?;
        // Note: The code assumes owner==close_authority
        //       If this is not the case then the caller should set close_authority accordingly before making the transfer
        if token_account_data.close_authority.is_some() {
            set_spl_token_account_authority(
                governed_token_info,
                governed_token_owner_info,
                token_governance_info.key,
                AuthorityType::CloseAccount,
                spl_token_info,
            )?;
        }
    } else {
        assert_spl_token_owner_is_signer(governed_token_info, governed_token_owner_info)?;
    }

    Ok(())
}
