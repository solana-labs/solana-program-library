//! Fund StartLiquidation instruction handler

use {
    crate::{common, fund_info::FundInfo},
    solana_farm_sdk::{
        fund::Fund,
        program::{account, clock},
    },
    solana_program::{
        account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
        sysvar::instructions,
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
        if !account::check_token_account_owner(user_fund_token_account, user_account.key)? {
            msg!("Error: Invalid Fund token account owner");
            return Err(ProgramError::IllegalOwner);
        }
        common::check_fund_token_mint(fund, fund_token_mint)?;

        if &instructions::id() != sysvar_account.key {
            return Err(ProgramError::UnsupportedSysvar);
        }
        if instructions::load_current_index_checked(sysvar_account)? != 0
            || instructions::load_instruction_at_checked(1, sysvar_account).is_ok()
        {
            msg!("Error: StartLiquidation must be single instruction in the transaction");
            return Err(ProgramError::InvalidArgument);
        }

        // check if liquidation can be started at this time
        let ft_supply_amount = account::get_token_supply(fund_token_mint)?;
        let last_admin_action_time = fund_info.get_admin_action_time()?;
        let curtime = clock::get_time()?;
        #[allow(clippy::if_same_then_else)]
        let allowed =
            // check if user is an admin or fund manager
            if user_account.key == &fund.admin_account {
                true
            }
            // check if user is roughly >= 60% stake holder
            else if ft_supply_amount > 0
                && account::get_token_balance(user_fund_token_account)? as f64 / ft_supply_amount as f64 >= 0.6
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
