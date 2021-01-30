//! Program state processor
use crate::{
    state::timelock_program::{TimelockProgram, TIMELOCK_VERSION},
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
pub fn process_init_timelock_program(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let program_info = next_account_info(account_info_iter)?;
    let rent = &Rent::from_account_info(next_account_info(account_info_iter)?)?;

    assert_rent_exempt(rent, program_info)?;
    let mut new_timelock_program: TimelockProgram = assert_uninitialized(program_info)?;
    new_timelock_program.version = TIMELOCK_VERSION;
    TimelockProgram::pack(new_timelock_program, &mut program_info.data.borrow_mut())?;

    Ok(())
}
