//! Program state processor
use crate::{
    state::timelock_set::{TimelockSet, TIMELOCK_SET_VERSION},
    utils::{assert_rent_exempt, assert_uninitialized},
};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    program_pack::Pack,
    pubkey::Pubkey,
    sysvar::{rent::Rent, Sysvar},
};

/// Create a new timelock program
pub fn process_add_signer(_program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let timelock_set_account_info = next_account_info(account_info_iter)?;
    let rent = &Rent::from_account_info(next_account_info(account_info_iter)?)?;

    assert_rent_exempt(rent, timelock_set_account_info)?;
    let mut new_timelock_set: TimelockSet = assert_uninitialized(timelock_set_account_info)?;
    TimelockSet::pack(
        new_timelock_set,
        &mut timelock_set_account_info.data.borrow_mut(),
    )?;

    Ok(())
}
