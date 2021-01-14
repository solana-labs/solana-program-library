//! Program instruction processor

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    pubkey::Pubkey,
    sysvar::{clock::Clock, rent::Rent, Sysvar},
};

/// Instruction processor
pub fn process_instruction(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    _instruction_data: &[u8],
) -> ProgramResult {
    // Create in iterator to safety reference accounts in the slice
    let account_info_iter = &mut accounts.iter();

    // The first account is the clock sysvar
    let clock_sysvar_info = next_account_info(account_info_iter)?;
    // The second account is the rent sysvar
    let rent_sysvar_info = next_account_info(account_info_iter)?;

    // Deserialize the account into a clock struct
    let clock = Clock::from_account_info(&clock_sysvar_info)?;

    // Deserialize the account into a rent struct
    let rent = Rent::from_account_info(&rent_sysvar_info)?;

    // Note: `format!` can be very expensive, use cautiously
    msg!("{:?}", clock);
    // Can't print `exemption_threshold` because BPF does not support printing floats
    msg!(
        "Rent: lamports_per_byte_year: {:?}, burn_percent: {:?}",
        rent.lamports_per_byte_year,
        rent.burn_percent
    );

    Ok(())
}
