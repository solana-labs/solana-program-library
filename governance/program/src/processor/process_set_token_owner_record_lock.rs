//! Program state processor

use {
    crate::{
        error::GovernanceError,
        state::{
            enums::GovernanceAccountType,
            token_owner_record::{get_token_owner_record_data, TokenOwnerRecordLock},
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

    let token_owner_record_info = next_account_info(account_info_iter)?; // 0
    let token_owner_record_lock_authority_info = next_account_info(account_info_iter)?; // 1
    let payer_info = next_account_info(account_info_iter)?; // 2
    let system_info = next_account_info(account_info_iter)?; // 3

    let rent = Rent::get()?;
    let clock = Clock::get()?;

    if !token_owner_record_lock_authority_info.is_signer {
        return Err(GovernanceError::TokenOwnerRecordLockAuthorityMustSign.into());
    }

    // TODO:
    // 1) Assert the authority is on the list for the given token
    // 2) Find existing by (authority,type) and update or insert new
    // 3) Trim expired locks
    // 4) Save as V2 and resize as needed
    // 5) Should expired lock be rejected ?

    let mut token_owner_record_data =
        get_token_owner_record_data(program_id, token_owner_record_info)?;

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

        // When the account is resized we have to change the type is V2 to preserve
        // the extra data
        if token_owner_record_data.account_type == GovernanceAccountType::TokenOwnerRecordV1 {
            token_owner_record_data.account_type = GovernanceAccountType::TokenOwnerRecordV2;
        }
    }

    token_owner_record_data.serialize(&mut token_owner_record_info.data.borrow_mut()[..])?;

    Ok(())
}
