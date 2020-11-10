//! Program instruction processor

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    info,
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvar::{self, clock::Clock, Sysvar},
};

/// Instruction processor
pub fn process_instruction(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    _instruction_data: &[u8],
) -> ProgramResult {
    // Create in iterator to safety reference accounts in the slice
    let account_info_iter = &mut accounts.iter();

    // As part of the program specification the first account is the clock
    // sysvar
    let clock_sysvar_info = next_account_info(account_info_iter)?;

    if *clock_sysvar_info.key != sysvar::clock::id() {
        // first account is not the clock sysvar
        return Err(ProgramError::InvalidArgument);
    }

    // Deserialize the account into a clock struct
    let clock = Clock::from_account_info(&clock_sysvar_info)?;

    // Note: `format!` can be very expensive, use cautiously
    info!(&format!("{:?}", clock));

    Ok(())
}
