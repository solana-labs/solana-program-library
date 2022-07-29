//! Fund StartLiquidation instruction handler

use {
    crate::{common, fund_info::FundInfo, user_info::UserInfo},
    solana_farm_sdk::{
        fund::Fund,
        program,
        program::{account, clock},
    },
    solana_program::{
        account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
    },
};

pub fn start_liquidation(fund: &Fund, accounts: &[AccountInfo]) -> ProgramResult {
    #[allow(clippy::deprecated_cfg_attr)]
    #[cfg_attr(rustfmt, rustfmt_skip)]
    if let [
        user_account,
        _fund_metadata,
        fund_info_account,
        fund_token_mint,
        user_info_account,
        user_fund_token_account,
        sysvar_account
        ] = accounts
    {
        // validate accounts
        msg!("Validate state and accounts");
        let mut fund_info = FundInfo::new(fund_info_account);
        if fund_info.get_liquidation_start_time()? > 0 {
            msg!("Error: Fund is already in liquidation state");
            return Err(ProgramError::Custom(516));
        }
        if !user_account.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }
        if !UserInfo::validate_account(fund, user_info_account, user_account.key) {
            msg!("Error: Invalid user info account");
            return Err(ProgramError::Custom(140));
        }
        if !account::check_token_account_owner(user_fund_token_account, user_account.key)? {
            msg!("Error: Invalid Fund token account owner");
            return Err(ProgramError::IllegalOwner);
        }
        common::check_fund_token_mint(fund, fund_token_mint)?;

        if !program::is_single_instruction(sysvar_account)? {
            msg!("Error: StartLiquidation must be single instruction in the transaction");
            return Err(ProgramError::InvalidArgument);
        }

        // check if liquidation can be started at this time
        let ft_supply_amount = common::get_fund_token_supply(fund_token_mint, &fund_info)?;
        let last_admin_action_time = fund_info.get_admin_action_time()?;
        let user_info = UserInfo::new(user_info_account);
        let curtime = clock::get_time()?;
        #[allow(clippy::if_same_then_else)]
        let allowed =
            // check if user is roughly >= 60% stake holder
            if ft_supply_amount > 0
                && common::get_fund_token_balance(user_fund_token_account, &user_info)? as f64 / ft_supply_amount as f64 >= 0.6
            {
                true
            }
            // check if no admin activity in the past 2 weeks
            else { 
                last_admin_action_time > 0 && curtime - last_admin_action_time >= 1209600
            };

        if !allowed {
            msg!("Error: Liquidation can be started if no admin actions performed in the next {} seconds", 1209600i64 - curtime + last_admin_action_time);
            msg!("Error: Liquidation can also be started by Fund admin or user with >= 60% stake");
            return Err(ProgramError::Custom(519));
        }

        // start liquidation
        msg!("Initiate liquidation");
        fund_info.set_liquidation_start_time(curtime)?;
        fund_info.set_liquidation_amount_usd(fund_info.get_current_assets_usd()?)?;
        fund_info.set_liquidation_amount_tokens(ft_supply_amount)?;

        Ok(())
    } else {
        Err(ProgramError::NotEnoughAccountKeys)
    }
}
