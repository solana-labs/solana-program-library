//! Program state processor

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    pubkey::Pubkey,
    rent::Rent,
    sysvar::Sysvar,
};
use spl_governance_tools::account::create_and_serialize_account_signed2;

use crate::state::native_treasury::{get_native_treasury_address_seeds, NativeTreasury};

/// Processes CreateNativeTreasury instruction
pub fn process_create_native_treasury(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let governance_info = next_account_info(account_info_iter)?; // 0
    let native_treasury_info = next_account_info(account_info_iter)?; // 1
    let payer_info = next_account_info(account_info_iter)?; // 2
    let system_info = next_account_info(account_info_iter)?; // 3

    let rent = Rent::get()?;

    // TODO: Assert is valid Governance

    let native_treasury_data = NativeTreasury {};

    create_and_serialize_account_signed2(
        payer_info,
        native_treasury_info,
        &native_treasury_data,
        &get_native_treasury_address_seeds(governance_info.key),
        program_id,
        system_info.key,
        system_info,
        &rent,
    )?;

    Ok(())
}
