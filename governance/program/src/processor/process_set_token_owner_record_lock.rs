//! Program state processor

use {
    crate::{
        error::GovernanceError,
        state::{
            enums::GovernanceAccountType,
            realm::get_realm_data,
            realm_config::get_realm_config_data_for_realm,
            token_owner_record::{get_token_owner_record_data_for_realm, TokenOwnerRecordLock},
        },
    },
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        clock::{Clock, UnixTimestamp},
        entrypoint::ProgramResult,
        pubkey::Pubkey,
        rent::Rent,
        sysvar::Sysvar,
    },
    spl_governance_tools::account::{extend_account_size, AccountMaxSize},
};

/// Processes SetTokenOwnerRecordLock instruction
pub fn process_set_token_owner_record_lock(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    lock_type: u8,
    expiry: Option<UnixTimestamp>,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let realm_info = next_account_info(account_info_iter)?; // 0
    let realm_config_info = next_account_info(account_info_iter)?; // 1
    let token_owner_record_info = next_account_info(account_info_iter)?; // 2
    let token_owner_record_lock_authority_info = next_account_info(account_info_iter)?; // 3
    let payer_info = next_account_info(account_info_iter)?; // 4
    let system_info = next_account_info(account_info_iter)?; // 5

    let rent = Rent::get()?;
    let clock = Clock::get()?;

    if !token_owner_record_lock_authority_info.is_signer {
        return Err(GovernanceError::TokenOwnerRecordLockAuthorityMustSign.into());
    }

    // Reject the lock if already expired
    if expiry.is_some() && clock.unix_timestamp > expiry.unwrap() {
        return Err(GovernanceError::ExpiredTokenOwnerRecordLock.into());
    }

    let realm_data = get_realm_data(program_id, realm_info)?;
    let realm_config_data =
        get_realm_config_data_for_realm(program_id, realm_config_info, realm_info.key)?;

    let mut token_owner_record_data = get_token_owner_record_data_for_realm(
        program_id,
        token_owner_record_info,
        &realm_config_data.realm,
    )?;

    if !realm_config_data
        .get_token_config(&realm_data, &token_owner_record_data.governing_token_mint)?
        .lock_authorities
        .contains(token_owner_record_lock_authority_info.key)
    {
        return Err(GovernanceError::InvalidTokenOwnerRecordLockAuthority.into());
    }

    // Trim existing locks
    token_owner_record_data.locks.retain(|lock| {
        // Remove existing lock for the authority and lock type we set
        if lock.lock_type == lock_type
            && lock.authority == *token_owner_record_lock_authority_info.key
        {
            false
        } else {
            // Retain only unexpired locks
            lock.expiry > Some(clock.unix_timestamp)
        }
    });

    // Add the new lock
    token_owner_record_data.locks.push(TokenOwnerRecordLock {
        lock_type,
        authority: *token_owner_record_lock_authority_info.key,
        expiry,
    });

    let token_owner_record_data_max_size = token_owner_record_data.get_max_size().unwrap();
    if token_owner_record_info.data_len() < token_owner_record_data_max_size {
        extend_account_size(
            token_owner_record_info,
            payer_info,
            token_owner_record_data_max_size,
            &rent,
            system_info,
        )?;

        // When the account is resized we have to change the type to V2 to preserve
        // the extra data
        if token_owner_record_data.account_type == GovernanceAccountType::TokenOwnerRecordV1 {
            token_owner_record_data.account_type = GovernanceAccountType::TokenOwnerRecordV2;
        }
    }

    token_owner_record_data.serialize(&mut token_owner_record_info.data.borrow_mut()[..])?;

    Ok(())
}
