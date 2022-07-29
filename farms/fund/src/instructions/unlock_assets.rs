//! Move funds from trading to withdrawal custody instruction handler

use {
    crate::common,
    solana_farm_sdk::{
        fund::{Fund, FundCustodyType},
        program::{account, pda},
        token::Token,
    },
    solana_program::{
        account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
    },
};

pub fn unlock_assets(fund: &Fund, accounts: &[AccountInfo], amount: u64) -> ProgramResult {
    #[allow(clippy::deprecated_cfg_attr)]
    #[cfg_attr(rustfmt, rustfmt_skip)]
    if let [
        _admin_account,
        fund_metadata,
        _fund_info_account,
        _multisig_account,
        fund_authority,
        _spl_token_program,
        wd_custody_account,
        wd_custody_metadata,
        trading_custody_account,
        trading_custody_metadata,
        custody_token_metadata
        ] = accounts
    {
        // validate params and accounts
        msg!("Validate state and accounts");
        if fund_authority.key != &fund.fund_authority {
            msg!("Error: Invalid Fund authority account");
            return Err(ProgramError::Custom(517));
        }

        let custody_token = account::unpack::<Token>(custody_token_metadata, "custody token")?;
        common::check_custody_account(
            &fund.fund_program_id,
            fund_metadata.key,
            &custody_token,
            custody_token_metadata,
            wd_custody_metadata,
            FundCustodyType::DepositWithdraw,
            wd_custody_account,
            None,
        )?;
        common::check_custody_account(
            &fund.fund_program_id,
            fund_metadata.key,
            &custody_token,
            custody_token_metadata,
            trading_custody_metadata,
            FundCustodyType::Trading,
            trading_custody_account,
            None,
        )?;

        // check if there are funds in w/d custody
        let trading_custody_balance = account::get_token_balance(trading_custody_account)?;
        let amount = if amount > 0 {
            amount
        } else {
            trading_custody_balance
        };
        if amount == 0 || amount < trading_custody_balance {
            msg!("Error: Not enough funds in trading custody");
            return Err(ProgramError::Custom(530));
        }

        // trandsfer tokens from trading to w/d custody
        msg!("Transfer funds to w/d custody");
        let seeds: &[&[&[u8]]] = &[&[
            b"fund_authority",
            fund.name.as_bytes(),
            &[fund.authority_bump],
        ]];
        pda::transfer_tokens_with_seeds(
            trading_custody_account,
            wd_custody_account,
            fund_authority,
            seeds,
            amount,
        )?;

        Ok(())
    } else {
        Err(ProgramError::NotEnoughAccountKeys)
    }
}
