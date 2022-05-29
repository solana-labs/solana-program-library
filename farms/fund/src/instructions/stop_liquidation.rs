//! Fund StopLiquidation instruction handler

use {
    crate::fund_info::FundInfo,
    solana_farm_sdk::fund::Fund,
    solana_program::{
        account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
    },
};

pub fn stop_liquidation(_fund: &Fund, accounts: &[AccountInfo]) -> ProgramResult {
    #[allow(clippy::deprecated_cfg_attr)]
    #[cfg_attr(rustfmt, rustfmt_skip)]
    if let [
        _admin_account,
        _fund_metadata,
        fund_info_account,
        _active_multisig_account
        ] = accounts 
    {
        // validate accounts
        msg!("Validate state and accounts");
        let mut fund_info = FundInfo::new(fund_info_account);
        if fund_info.get_liquidation_start_time()? == 0 {
            msg!("Error: Fund is not in liquidation state");
            return Err(ProgramError::Custom(518));
        }

        // stop liquidation
        msg!("Stop liquidation");
        fund_info.set_liquidation_start_time(0)?;
        fund_info.update_admin_action_time()
    } else {
        Err(ProgramError::NotEnoughAccountKeys)
    }
}
