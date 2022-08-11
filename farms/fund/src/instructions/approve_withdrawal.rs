//! Approve withdrawal from the Fund instruction handler

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

pub fn approve_withdrawal(fund: &Fund, accounts: &[AccountInfo], amount: u64) -> ProgramResult {
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
        user_withdrawal_token_account,
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
        if !account::check_token_account_owner(user_withdrawal_token_account, user_account.key)? {
            msg!("Error: Invalid withdrawal destination account owner");
            return Err(ProgramError::IllegalOwner);
        }
        common::check_fund_token_mint(fund, fund_token_mint)?;

        let custody_token = account::unpack::<Token>(custody_token_metadata, "custody token")?;
        common::check_wd_custody_accounts(
            &fund.fund_program_id,
            fund_metadata.key,
            &custody_token,
            custody_token_metadata,
            user_withdrawal_token_account,
            custody_account,
            custody_fees_account,
            custody_metadata,
            oracle_account,
        )?;

        let mut user_requests =
            account::unpack::<FundUserRequests>(user_requests_account, "user requests")?;
        common::check_user_requests_account(
            fund,
            &custody_token,
            &user_requests,
            user_account,
            user_requests_account,
        )?;

        // check if there are any pending requests
        if user_requests.withdrawal_request.amount == 0 {
            msg!("Error: No pending withdrawals found");
            return Err(ProgramError::Custom(526));
        }

        // compute withdrawal amount
        msg!("Compute withdrawal amount");
        let amount_with_fee = if amount == 0 {
            user_requests.withdrawal_request.amount
        } else {
            std::cmp::min(amount, user_requests.withdrawal_request.amount)
        };
        let mut user_info = UserInfo::new(user_info_account);
        let user_fund_token_balance =
            common::get_fund_token_balance(user_fund_token_account, &user_info)?;
        if amount_with_fee == 0 || amount_with_fee > user_fund_token_balance {
            msg!("Error: Insufficient user funds");
            return Err(ProgramError::InsufficientFunds);
        }

        // check if last assets update was not too long ago,
        // stale value may lead to incorrect amount of tokens received
        common::check_assets_update_time(
            fund_info.get_assets_update_time()?,
            fund_info.get_assets_max_update_age_sec()?,
        )?;

        // compute nominal value of withdrawn tokens and check against the limit
        msg!("Compute assets value. amount_with_fee: {}", amount_with_fee);
        let ft_supply_amount = common::get_fund_token_supply(fund_token_mint, &fund_info)?;
        if amount_with_fee > ft_supply_amount {
            msg!("Error: Insufficient Fund supply amount");
            return Err(ProgramError::InsufficientFunds);
        }
        // ft_supply_amount > 0
        let withdrawal_value_usd =
            fund_info.get_current_assets_usd()? * amount_with_fee as f64 / ft_supply_amount as f64;

        // no individual withdrawal limit check (discretion of the Fund manager)

        // compute tokens to transfer
        msg!(
            "Compute tokens to transfer. withdrawal_value_usd: {}",
            withdrawal_value_usd
        );
        let tokens_to_remove = account::get_asset_value_tokens(
            withdrawal_value_usd,
            custody_token.decimals,
            custody_token.oracle_type,
            oracle_account,
            fund_info.get_assets_max_price_error()?,
            fund_info.get_assets_max_price_age_sec()?,
        )?;

        // 0 <= fund_fee <= 1
        let fund_fee = fund_info.get_withdrawal_fee()?;
        let (fee_numerator, fee_denominator) = math::get_fee_parts(fund_fee);
        let fee_tokens = math::checked_as_u64(math::checked_div(
            math::checked_mul(tokens_to_remove as u128, fee_numerator as u128)?,
            fee_denominator as u128,
        )?)?;
        let tokens_to_tranfer = math::checked_sub(tokens_to_remove, fee_tokens)?;
        if tokens_to_tranfer == 0 {
            msg!("Error: Withdrawal amount is too small");
            return Err(ProgramError::InsufficientFunds);
        }

        if tokens_to_remove > account::get_token_balance(custody_account)? {
            msg!("Error: Withdrawal for this amount couldn't be completed at this time. Contact Fund administrator.");
            return Err(ProgramError::InsufficientFunds);
        }

        // transfer tokens from custody to the user
        msg!(
            "Transfer tokens to user wallet. tokens_to_tranfer: {}, fee_tokens: {}",
            tokens_to_tranfer,
            fee_tokens,
        );
        let seeds: &[&[&[u8]]] = &[&[
            b"fund_authority",
            fund.name.as_bytes(),
            &[fund.authority_bump],
        ]];
        pda::transfer_tokens_with_seeds(
            custody_account,
            user_withdrawal_token_account,
            fund_authority,
            seeds,
            tokens_to_tranfer,
        )?;
        if fee_tokens > 0 {
            pda::transfer_tokens_with_seeds(
                custody_account,
                custody_fees_account,
                fund_authority,
                seeds,
                fee_tokens,
            )?;
        }

        // burn Fund tokens from user
        msg!(
            "Burn Fund tokens from the user. amount_with_fee {}",
            amount_with_fee
        );
        let (amount_to_burn, amount_to_reduce) = if fund_info.get_issue_virtual_tokens()? {
            let token_balance = account::get_token_balance(user_fund_token_account)?;
            let amount_to_burn = std::cmp::min(amount_with_fee, token_balance);
            let amount_to_reduce = math::checked_sub(amount_with_fee, amount_to_burn)?;
            (amount_to_burn, amount_to_reduce)
        } else {
            let amount_to_reduce =
                std::cmp::min(amount_with_fee, user_info.get_virtual_tokens_balance()?);
            let amount_to_burn = math::checked_sub(amount_with_fee, amount_to_reduce)?;
            (amount_to_burn, amount_to_reduce)
        };
        pda::burn_tokens_with_seeds(
            user_fund_token_account,
            fund_token_mint,
            fund_authority,
            seeds,
            amount_to_burn,
        )?;
        user_info.set_virtual_tokens_balance(math::checked_sub(
            user_info.get_virtual_tokens_balance()?,
            amount_to_reduce,
        )?)?;
        fund_info.set_virtual_tokens_supply(math::checked_sub(
            fund_info.get_virtual_tokens_supply()?,
            amount_to_reduce,
        )?)?;

        // update stats
        msg!("Update Fund stats");
        let current_assets_usd = fund_info.get_current_assets_usd()?;
        let new_assets = if current_assets_usd > withdrawal_value_usd {
            current_assets_usd - withdrawal_value_usd
        } else {
            0.0
        };
        fund_info
            .set_amount_removed_usd(fund_info.get_amount_removed_usd()? + withdrawal_value_usd)?;
        fund_info.set_current_assets_usd(new_assets)?;

        // update user stats
        msg!("Update user stats");
        user_requests.last_withdrawal.time = user_requests.withdrawal_request.time;
        user_requests.last_withdrawal.amount = amount_with_fee;
        user_requests.withdrawal_request.time = 0;
        user_requests.withdrawal_request.amount = 0;
        user_requests.deny_reason = ArrayString64::default();
        user_requests.pack(*user_requests_account.try_borrow_mut_data()?)?;

        Ok(())
    } else {
        Err(ProgramError::NotEnoughAccountKeys)
    }
}
