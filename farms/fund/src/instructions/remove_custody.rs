//! Fund RemoveCustody instruction handler

use {
    crate::{common, fund_info::FundInfo},
    solana_farm_sdk::{
        fund::{Fund, FundAssetType, FundCustodyType},
        program::{account, pda},
        token::Token,
        traits::Packed,
    },
    solana_program::{
        account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
    },
};

pub fn remove_custody(
    fund: &Fund,
    accounts: &[AccountInfo],
    target_hash: u64,
    custody_type: FundCustodyType,
) -> ProgramResult {
    #[allow(clippy::deprecated_cfg_attr)]
    #[cfg_attr(rustfmt, rustfmt_skip)]
    if let [
        admin_account,
        fund_metadata,
        fund_info_account,
        _active_multisig_account,
        fund_multisig_account,
        fund_authority,
        _system_program,
        _spl_token_program,
        custodies_assets_info,
        custody_account,
        custody_fees_account,
        custody_metadata,
        custody_token_metadata
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

        let custody_token = account::unpack::<Token>(custody_token_metadata, "custody token")?;
        let is_vault_token =
            custody_token.name.len() > 3 && ["LP.", "VT."].contains(&&custody_token.name[..3]);
        common::check_custody_account(
            &fund.fund_program_id,
            fund_metadata.key,
            &custody_token,
            custody_token_metadata,
            custody_metadata,
            custody_type,
            custody_account,
            Some(custody_fees_account.key),
        )?;

        if account::get_token_balance(custody_account)? > 0 ||
            account::get_token_balance(custody_fees_account)? > 0{
            msg!("Custody token accounts must be empty");
            return Err(ProgramError::Custom(539));
        }

        // close accounts
        msg!("Close custody token accounts");
        let seeds: &[&[&[u8]]] = &[&[
            b"fund_authority",
            fund.name.as_bytes(),
            &[fund.authority_bump],
        ]];
        pda::close_token_account_with_seeds(admin_account, custody_account, fund_authority, seeds)?;

        let seeds: &[&[&[u8]]] = &[&[
            b"multisig",
            fund.name.as_bytes(),
            &[fund.multisig_bump],
        ]];
        pda::close_token_account_with_seeds(admin_account, custody_fees_account, fund_multisig_account, seeds)?;

        msg!("Close custody metadata account");
        account::close_system_account(admin_account, custody_metadata, &fund.fund_program_id)?;

        // if this is non-vault token custody then assets stats must be reset
        if !is_vault_token {
            // update assets tracking account
            msg!("Update Fund assets account");
            let mut fund_assets = common::check_and_get_fund_assets_account(
                fund,
                custodies_assets_info,
                FundAssetType::Custody,
            )?;
            fund_assets.current_hash = 0;
            fund_assets.target_hash = target_hash;
            fund_assets.cycle_start_time = 0;
            fund_assets.cycle_end_time = 0;
            fund_assets.pack(*custodies_assets_info.try_borrow_mut_data()?)?;
        }

        // update fund stats
        msg!("Update Fund stats");
        fund_info.update_admin_action_time()
    } else {
        Err(ProgramError::NotEnoughAccountKeys)
    }
}
