//! Program state processor

use {
    crate::{
        error::GovernanceError,
        state::{
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
};

/// Processes SetTokenOwnerRecordLock instruction
pub fn process_set_token_owner_record_lock(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    lock_id: u8,
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

    // Remove expired locks and matching the authority and lock type we set
    token_owner_record_data.trim_locks(
        clock.unix_timestamp,
        lock_id,
        token_owner_record_lock_authority_info.key,
    );

    // Add the new lock
    token_owner_record_data.locks.push(TokenOwnerRecordLock {
        lock_id,
        authority: *token_owner_record_lock_authority_info.key,
        expiry,
    });

    token_owner_record_data.serialize_with_resize(
        token_owner_record_info,
        payer_info,
        system_info,
        &rent,
    )?;

    Ok(())
}
