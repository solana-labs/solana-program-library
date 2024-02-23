//! Program state processor

use {
    crate::{
        error::GovernanceError,
        state::{
            realm::{get_realm_data_for_authority, SetRealmConfigItemArgs},
            realm_config::get_realm_config_data_for_realm,
        },
        tools::structs::SetConfigItemActionType,
    },
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint::ProgramResult,
        pubkey::Pubkey,
        rent::Rent,
        sysvar::Sysvar,
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
                SetConfigItemActionType::Add => {
                    if token_config.lock_authorities.contains(&authority) {
                        return Err(
                            GovernanceError::TokenOwnerRecordLockAuthorityAlreadyExists.into()
                        );
                    }

                    token_config.lock_authorities.push(authority);
                }
                SetConfigItemActionType::Remove => {
                    if let Some(lock_authority_index) = token_config
                        .lock_authorities
                        .iter()
                        .position(|lock_authority| lock_authority == &authority)
                    {
                        token_config.lock_authorities.remove(lock_authority_index);
                    } else {
                        return Err(GovernanceError::TokenOwnerRecordLockAuthorityNotFound.into());
                    }
                }
            }
        }
    }

    realm_config_data.serialize(
        program_id,
        realm_config_info,
        payer_info,
        system_info,
        &rent,
    )?;

    Ok(())
}
