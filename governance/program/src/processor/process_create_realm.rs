//! Program state processor

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    pubkey::Pubkey,
    rent::Rent,
    sysvar::Sysvar,
};
use spl_governance_tools::account::create_and_serialize_account_signed;

use crate::{
    error::GovernanceError,
    state::{
        enums::GovernanceAccountType,
        realm::{
            assert_valid_realm_config_args, get_governing_token_holding_address_seeds,
            get_realm_address_seeds, Realm, RealmConfig, RealmConfigArgs,
        },
        realm_config::{get_realm_config_address_seeds, RealmConfigAccount},
    },
    tools::spl_token::create_spl_token_account_signed,
};

/// Processes CreateRealm instruction
pub fn process_create_realm(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    name: String,
    config_args: RealmConfigArgs,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let realm_info = next_account_info(account_info_iter)?; // 0
    let realm_authority_info = next_account_info(account_info_iter)?; // 1
    let governance_token_mint_info = next_account_info(account_info_iter)?; // 2
    let governance_token_holding_info = next_account_info(account_info_iter)?; // 3
    let payer_info = next_account_info(account_info_iter)?; // 4
    let system_info = next_account_info(account_info_iter)?; // 5
    let spl_token_info = next_account_info(account_info_iter)?; // 6

    let rent_sysvar_info = next_account_info(account_info_iter)?; // 7
    let rent = &Rent::from_account_info(rent_sysvar_info)?;

    if !realm_info.data_is_empty() {
        return Err(GovernanceError::RealmAlreadyExists.into());
    }

    assert_valid_realm_config_args(&config_args)?;

    create_spl_token_account_signed(
        payer_info,
        governance_token_holding_info,
        &get_governing_token_holding_address_seeds(realm_info.key, governance_token_mint_info.key),
        governance_token_mint_info,
        realm_info,
        program_id,
        system_info,
        spl_token_info,
        rent_sysvar_info,
        rent,
    )?;

    let council_token_mint_address = if config_args.use_council_mint {
        let council_token_mint_info = next_account_info(account_info_iter)?; // 8
        let council_token_holding_info = next_account_info(account_info_iter)?; // 9

        create_spl_token_account_signed(
            payer_info,
            council_token_holding_info,
            &get_governing_token_holding_address_seeds(realm_info.key, council_token_mint_info.key),
            council_token_mint_info,
            realm_info,
            program_id,
            system_info,
            spl_token_info,
            rent_sysvar_info,
            rent,
        )?;

        Some(*council_token_mint_info.key)
    } else {
        None
    };

    if config_args.use_community_voter_weight_addin {
        let realm_config_info = next_account_info(account_info_iter)?; // 10
        let community_voter_weight_addin_info = next_account_info(account_info_iter)?; //11

        let realm_config_data = RealmConfigAccount {
            account_type: GovernanceAccountType::RealmConfig,
            realm: *realm_info.key,
            community_voter_weight_addin: Some(*community_voter_weight_addin_info.key),
            community_max_vote_weight_addin: None,
            council_voter_weight_addin: None,
            council_max_vote_weight_addin: None,
            reserved: [0; 128],
        };

        create_and_serialize_account_signed::<RealmConfigAccount>(
            payer_info,
            realm_config_info,
            &realm_config_data,
            &get_realm_config_address_seeds(realm_info.key),
            program_id,
            system_info,
            rent,
        )?;
    }

    let realm_data = Realm {
        account_type: GovernanceAccountType::Realm,
        community_mint: *governance_token_mint_info.key,

        name: name.clone(),
        reserved: [0; 8],
        authority: Some(*realm_authority_info.key),
        config: RealmConfig {
            council_mint: council_token_mint_address,
            reserved: [0; 7],
            community_mint_max_vote_weight_source: config_args
                .community_mint_max_vote_weight_source,
            min_community_tokens_to_create_governance: config_args
                .min_community_tokens_to_create_governance,
            use_community_voter_weight_addin: config_args.use_community_voter_weight_addin,
        },
    };

    create_and_serialize_account_signed::<Realm>(
        payer_info,
        realm_info,
        &realm_data,
        &get_realm_address_seeds(&name),
        program_id,
        system_info,
        rent,
    )?;

    Ok(())
}
