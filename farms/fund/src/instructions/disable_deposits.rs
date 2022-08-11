//! Fund DisableDeposits instruction handler

use {
    crate::fund_info::FundInfo,
    solana_farm_sdk::fund::Fund,
    solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, msg},
};

pub fn disable_deposits(
    _fund: &Fund,
    fund_info: &mut FundInfo,
    _accounts: &[AccountInfo],
) -> ProgramResult {
    msg!("Disable deposits to the Fund");

    fund_info.set_deposit_start_time(0)?;
    fund_info.set_deposit_end_time(0)?;
    fund_info.set_deposit_approval_required(true)
}
