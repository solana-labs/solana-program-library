//! Program state processor

use {
    crate::{
        error::GovernanceError,
        state::{
            realm::{get_realm_data_for_authority, SetRealmConfigItemArgs},
            realm_config::{
                get_realm_config_address_seeds, get_realm_config_data_for_realm, RealmConfigAccount,
            },
        },
        tools::structs::SetItemActionType,
    },
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint::ProgramResult,
        pubkey::Pubkey,
        rent::Rent,
        sysvar::Sysvar,
    },
    spl_governance_tools::account::{
        create_and_serialize_account_signed, extend_account_size, AccountMaxSize,
    },
};

/// Processes SetRealmConfigItem instruction
pub fn process_set_realm_config_item(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: SetRealmConfigItemArgs,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let realm_info = next_account_info(account_info_iter)?; // 0
    let realm_config_info = next_account_info(account_info_iter)?; // 1
    let realm_authority_info = next_account_info(account_info_iter)?; // 2
    let payer_info = next_account_info(account_info_iter)?; // 3
    let system_info = next_account_info(account_info_iter)?; // 4

    let rent = Rent::get()?;

    let realm_data =
        get_realm_data_for_authority(program_id, realm_info, realm_authority_info.key)?;

    if !realm_authority_info.is_signer {
        return Err(GovernanceError::RealmAuthorityMustSign.into());
    }

    let mut realm_config_data =
        get_realm_config_data_for_realm(program_id, realm_config_info, realm_info.key)?;

    match args {
        SetRealmConfigItemArgs::TokenOwnerRecordLockAuthority {
            action,
            governing_token_mint,
            authority,
        } => {
            let token_config =
                realm_config_data.get_token_config_mut(&realm_data, &governing_token_mint)?;

            match action {
                SetItemActionType::Add => {
                    // TODO: Check for duplicates
                    token_config.lock_authorities.push(authority);
                }
                SetItemActionType::Remove => {
                    token_config
                        .lock_authorities
                        .retain(|lock_authority| lock_authority != &authority);
                }
            }
        }
    }

    // Update or create RealmConfigAccount
    if realm_config_info.data_is_empty() {
        // For older Realm accounts (pre program V3) RealmConfigAccount might not exist
        // yet and we have to create it

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
        let realm_config_max_size = realm_config_data.get_max_size().unwrap();
        if realm_config_info.data_len() < realm_config_max_size {
            extend_account_size(
                realm_config_info,
                payer_info,
                realm_config_max_size,
                &rent,
                system_info,
            )?;
        }

        borsh::to_writer(
            &mut realm_config_info.data.borrow_mut()[..],
            &realm_config_data,
        )?;
    }

    Ok(())
}
