//! Program state processor

use {
    crate::{error::GovernanceError, state::token_owner_record::get_token_owner_record_data},
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        clock::Clock,
        entrypoint::ProgramResult,
        pubkey::Pubkey,
        sysvar::Sysvar,
    },
};

/// Processes RelinquishTokenOwnerRecordLocks instruction
pub fn process_relinquish_token_owner_record_locks(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    lock_id: Option<u8>,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let token_owner_record_info = next_account_info(account_info_iter)?; // 0
    let token_owner_record_lock_authority_info = next_account_info(account_info_iter)?; // 1

    let clock = Clock::get()?;

    if !token_owner_record_lock_authority_info.is_signer {
        return Err(GovernanceError::TokenOwnerRecordLockAuthorityMustSign.into());
    }

    let mut token_owner_record_data =
        get_token_owner_record_data(program_id, token_owner_record_info)?;

    // Remove the lock
    token_owner_record_data
        .remove_lock(lock_id.unwrap(), token_owner_record_lock_authority_info.key)?;

    // Trim expired locks
    token_owner_record_data.remove_expired_locks(clock.unix_timestamp);

    token_owner_record_data.serialize(&mut token_owner_record_info.data.borrow_mut()[..])?;

    Ok(())
}
