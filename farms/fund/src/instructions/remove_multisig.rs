//! Fund RemoveMultisig instruction handler

use {
    crate::fund_info::FundInfo,
    solana_farm_sdk::{fund::Fund, program::account},
    solana_program::{
        account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
    },
};

pub fn remove_multisig(fund: &Fund, accounts: &[AccountInfo]) -> ProgramResult {
    #[allow(clippy::deprecated_cfg_attr)]
    #[cfg_attr(rustfmt, rustfmt_skip)]
    if let [
        admin_account,
        _fund_metadata,
        fund_info_account,
        _active_multisig_account,
        fund_multisig_account,
        ] = accounts
    {
        msg!("Close multisig account");
        account::close_system_account(admin_account, fund_multisig_account, &fund.fund_program_id)?;

        // update fund stats
        msg!("Update Fund stats");
        let mut fund_info = FundInfo::new(fund_info_account);
        fund_info.update_admin_action_time()
    } else {
        Err(ProgramError::NotEnoughAccountKeys)
    }
}
