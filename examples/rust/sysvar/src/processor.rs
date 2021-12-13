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

    // Get the clock sysvar via syscall
    let clock_via_sysvar = Clock::get()?;
    // Or deserialize the account into a clock struct
    let clock_sysvar_info = next_account_info(account_info_iter)?;
    let clock_via_account = Clock::from_account_info(clock_sysvar_info)?;
    // Both produce the same sysvar
    assert_eq!(clock_via_sysvar, clock_via_account);
    // Note: `format!` can be very expensive, use cautiously
    msg!("{:?}", clock_via_sysvar);

    // Get the rent sysvar via syscall
    let rent_via_sysvar = Rent::get()?;
    // Or deserialize the account into a rent struct
    let rent_sysvar_info = next_account_info(account_info_iter)?;
    let rent_via_account = Rent::from_account_info(rent_sysvar_info)?;
    // Both produce the same sysvar
    assert_eq!(rent_via_sysvar, rent_via_account);
    // Can't print `exemption_threshold` because BPF does not support printing floats
    msg!(
        "Rent: lamports_per_byte_year: {:?}, burn_percent: {:?}",
        rent_via_sysvar.lamports_per_byte_year,
        rent_via_sysvar.burn_percent
    );

    Ok(())
}
