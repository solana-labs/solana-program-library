//! Program state processor

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    pubkey::Pubkey,
};

/// Processes RemoveTokenOwnerRecordLock instruction
pub fn process_remove_token_owner_record_lock(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    _lock_type: u8,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let _token_owner_record_info = next_account_info(account_info_iter)?; // 0
    let _token_owner_record_lock_authority_info = next_account_info(account_info_iter)?; // 1

    Ok(())
}
