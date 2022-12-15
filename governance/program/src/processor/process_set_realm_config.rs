//! Program state processor

use borsh::BorshSerialize;
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
        realm::{assert_valid_realm_config_args, get_realm_data_for_authority, RealmConfigArgs},
        realm_config::{
            get_realm_config_address_seeds, get_realm_config_data_for_realm,
            resolve_governing_token_config, RealmConfigAccount,
        },
    },
};

/// Processes SetRealmConfig instruction
pub fn process_set_realm_config(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    realm_config_args: RealmConfigArgs,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let realm_info = next_account_info(account_info_iter)?; // 0
    let realm_authority_info = next_account_info(account_info_iter)?; // 1

    let mut realm_data =
        get_realm_data_for_authority(program_id, realm_info, realm_authority_info.key)?;

    if !realm_authority_info.is_signer {
        return Err(GovernanceError::RealmAuthorityMustSign.into());
    }

    // Note: Config change leaves voting proposals in unpredictable state and it's DAOs responsibility
    // to ensure the changes are made when there are no proposals in voting state
    // For example changing voter-weight or max-voter-weight addin could accidentally make proposals to succeed which would otherwise be defeated

    assert_valid_realm_config_args(&realm_config_args)?;

    // Setup council
    if realm_config_args.use_council_mint {
        let council_token_mint_info = next_account_info(account_info_iter)?; // 2
        let _council_token_holding_info = next_account_info(account_info_iter)?; // 3

        // Council mint can only be at present set to None (removed) and changing it to other mint is not supported
        // It might be implemented in future versions but it needs careful planning
        // It can potentially open a can of warms like what happens with existing deposits or pending proposals
        if let Some(council_token_mint) = realm_data.config.council_mint {
            // Council mint can't be changed to different one
            if council_token_mint != *council_token_mint_info.key {
                return Err(GovernanceError::RealmCouncilMintChangeIsNotSupported.into());
            }
        } else {
            // Council mint can't be restored (changed from None)
            return Err(GovernanceError::RealmCouncilMintChangeIsNotSupported.into());
        }
    } else {
        // Remove council mint from realm
        // Note: In the current implementation this also makes it impossible to withdraw council tokens
        realm_data.config.council_mint = None;
    }

    let system_info = next_account_info(account_info_iter)?; // 4

    let realm_config_info = next_account_info(account_info_iter)?; // 5
    let mut realm_config_data =
        get_realm_config_data_for_realm(program_id, realm_config_info, realm_info.key)?;

    realm_config_data.assert_can_change_config(&realm_config_args)?;

    // Setup configs for tokens (plugins and token types)

    // 6, 7
    let community_token_config = resolve_governing_token_config(
        account_info_iter,
        &realm_config_args.community_token_config_args,
    )?;

    // 8, 9
    let council_token_config = resolve_governing_token_config(
        account_info_iter,
        &realm_config_args.council_token_config_args,
    )?;

    realm_config_data.community_token_config = community_token_config;
    realm_config_data.council_token_config = council_token_config;

    // Update or create RealmConfigAccount
    if realm_config_info.data_is_empty() {
        // For older Realm accounts (pre program V3) RealmConfigAccount might not exist yet and we have to create it

        // We need the payer to pay for the new account if it's created
        let payer_info = next_account_info(account_info_iter)?; // 10
        let rent = Rent::get()?;

        create_and_serialize_account_signed::<RealmConfigAccount>(
            payer_info,
            realm_config_info,
            &realm_config_data,
            &get_realm_config_address_seeds(realm_info.key),
            program_id,
            system_info,
            &rent,
            0,
        )?;
    } else {
        realm_config_data.serialize(&mut *realm_config_info.data.borrow_mut())?;
    }

    // Update RealmConfig (Realm.config field)
    realm_data.config.community_mint_max_voter_weight_source =
        realm_config_args.community_mint_max_voter_weight_source;

    realm_data.config.min_community_weight_to_create_governance =
        realm_config_args.min_community_weight_to_create_governance;

    realm_data.config.legacy1 = 0;
    realm_data.config.legacy2 = 0;

    realm_data.serialize(&mut *realm_info.data.borrow_mut())?;

    Ok(())
}
