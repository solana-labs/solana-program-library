//! Approve deposit to the Fund instruction handler

use {
    crate::{common, fund_info::FundInfo, user_info::UserInfo},
    solana_farm_sdk::{
        fund::{Fund, FundUserRequests},
        math,
        program::{account, pda},
        string::ArrayString64,
        token::Token,
        traits::Packed,
    },
    solana_program::{
        account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
    },
};

pub fn approve_deposit(fund: &Fund, accounts: &[AccountInfo], amount: u64) -> ProgramResult {
    #[allow(clippy::deprecated_cfg_attr)]
    #[cfg_attr(rustfmt, rustfmt_skip)]
    if let [
        _admin_account,
        fund_metadata,
        fund_info_account,
        _multisig_account,
        fund_authority,
        _spl_token_program,
        fund_token_mint,
        user_account,
        user_info_account,
        user_requests_account,
        user_deposit_token_account,
        user_fund_token_account,
        custody_account,
        custody_fees_account,
        custody_metadata,
        custody_token_metadata,
        oracle_account
        ] = accounts
    {
        // validate params and accounts
        msg!("Validate state and accounts");
        let mut fund_info = FundInfo::new(fund_info_account);
        if fund_info.get_liquidation_start_time()? > 0 {
            msg!("Error: Fund is in liquidation state");
            return Err(ProgramError::Custom(516));
        }
        if fund_authority.key != &fund.fund_authority {
            msg!("Error: Invalid Fund authority account");
            return Err(ProgramError::Custom(517));
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

        let custody_token = account::unpack::<Token>(custody_token_metadata, "custody token")?;
        common::check_wd_custody_accounts(
            &fund.fund_program_id,
            fund_metadata.key,
            &custody_token,
            custody_token_metadata,
            user_deposit_token_account,
            custody_account,
            custody_fees_account,
            custody_metadata,
            oracle_account,
        )?;

        let mut user_requests = account::unpack::<FundUserRequests>(user_requests_account, "user requests")?;
        common::check_user_requests_account(
            fund,
            &custody_token,
            &user_requests,
            user_account,
            user_requests_account,
        )?;

        // check if there are any pending requests
        if user_requests.deposit_request.amount == 0 {
            msg!("Error: No pending deposits found");
            return Err(ProgramError::Custom(525));
        }

        // compute deposit amount and fees
        msg!("Compute deposit amount and fees");
        let user_token_balance = account::get_token_balance(user_deposit_token_account)?;
        let amount_with_fee = if amount == 0 {
            user_requests.deposit_request.amount
        } else {
            std::cmp::min(amount, user_requests.deposit_request.amount)
        };
        // 0 <= fund_fee <= 1
        let fund_fee = fund_info.get_deposit_fee()?;
        let (fee_numerator, fee_denominator) = math::get_fee_parts(fund_fee);
        let deposit_fee = math::checked_as_u64(math::checked_div(
            math::checked_mul(amount_with_fee as u128, fee_numerator as u128)?,
            fee_denominator as u128,
        )?)?;
        let deposit_amount = amount_with_fee.checked_sub(deposit_fee).unwrap();
        if deposit_amount == 0 || deposit_amount > user_token_balance {
            msg!("Error: Insufficient user funds");
            return Err(ProgramError::InsufficientFunds);
        }

        // compute nominal value of deposited tokens and check against the limit
        msg!("Compute assets value. amount_with_fee: {}", amount_with_fee);
        let deposit_value_usd = account::get_asset_value_usd(
            deposit_amount,
            custody_token.decimals,
            custody_token.oracle_type,
            oracle_account,
            fund_info.get_assets_max_price_error()?,
            fund_info.get_assets_max_price_age_sec()?,
        )?;

        // no individual deposit limit check (discretion of the Fund manager)

        msg!(
            "Deposit tokens into custody. deposit_amount: {}, deposit_value_usd: {}",
            deposit_amount,
            deposit_value_usd
        );

        // check for total asset amount limit
        common::check_assets_limit_usd(&fund_info, deposit_value_usd)?;

        // check if last assets update was not too long ago,
        // stale value may lead to incorrect amount of fund tokens minted
        common::check_assets_update_time(
            fund_info.get_assets_update_time()?,
            fund_info.get_assets_max_update_age_sec()?,
        )?;

        // transfer funds
        let seeds: &[&[&[u8]]] = &[&[
            b"fund_authority",
            fund.name.as_bytes(),
            &[fund.authority_bump],
        ]];
        pda::transfer_tokens_with_seeds(
            user_deposit_token_account,
            custody_account,
            fund_authority,
            seeds,
            deposit_amount,
        )?;
        if deposit_fee > 0 {
            pda::transfer_tokens_with_seeds(
                user_deposit_token_account,
                custody_fees_account,
                fund_authority,
                seeds,
                deposit_fee,
            )?;
        }

        // mint Fund tokens to user
        let current_assets_usd = fund_info.get_current_assets_usd()?;
        let ft_supply_amount = common::get_fund_token_supply(fund_token_mint, &fund_info)?;
        let ft_to_mint = common::get_fund_token_to_mint_amount(
            current_assets_usd,
            deposit_amount,
            deposit_value_usd,
            ft_supply_amount,
        )?;
        msg!(
                "Mint Fund tokens to the user. ft_to_mint: {}, ft_supply_amount: {}, current_assets_usd: {}",
                ft_to_mint, ft_supply_amount,
                current_assets_usd
            );
        if ft_to_mint == 0 {
            msg!("Error: Deposit instruction didn't result in Fund tokens mint");
            return Err(ProgramError::Custom(170));
        }
        
        if fund_info.get_issue_virtual_tokens()? {
            let mut user_info = UserInfo::new(user_info_account);
            user_info.set_virtual_tokens_balance(math::checked_add(
                user_info.get_virtual_tokens_balance()?,
                ft_to_mint,
            )?)?;
            fund_info.set_virtual_tokens_supply(math::checked_add(
                fund_info.get_virtual_tokens_supply()?,
                ft_to_mint,
            )?)?;
        } else {
            pda::mint_to_with_seeds(
                user_fund_token_account,
                fund_token_mint,
                fund_authority,
                seeds,
                ft_to_mint,
            )?;
        }

        // update stats
        msg!("Update Fund stats");
        fund_info
            .set_amount_invested_usd(fund_info.get_amount_invested_usd()? + deposit_value_usd)?;
        fund_info.set_current_assets_usd(current_assets_usd + deposit_value_usd)?;

        // update user stats
        msg!("Update user stats");
        user_requests.last_deposit.time = user_requests.deposit_request.time;
        user_requests.last_deposit.amount = amount_with_fee;
        user_requests.deposit_request.time = 0;
        user_requests.deposit_request.amount = 0;
        user_requests.deny_reason = ArrayString64::default();
        user_requests.pack(*user_requests_account.try_borrow_mut_data()?)?;

        Ok(())
    } else {
        Err(ProgramError::NotEnoughAccountKeys)
    }
}
