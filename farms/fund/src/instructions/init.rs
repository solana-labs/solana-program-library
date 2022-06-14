//! Fund Init instruction handler

use {
    crate::fund_info::FundInfo,
    solana_farm_sdk::{
        fund::{Fund, FundAssetType, FundAssets},
        program::{account, pda},
        token::Token,
        traits::Packed,
    },
    solana_program::{
        account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
    },
};

pub fn init(fund: &Fund, accounts: &[AccountInfo], _step: u64) -> ProgramResult {
    #[allow(clippy::deprecated_cfg_attr)]
    #[cfg_attr(rustfmt, rustfmt_skip)]
    if let [
        admin_account,
        _fund_metadata,
        fund_info_account,
        _multisig_account,
        fund_authority,
        fund_program,
        _system_program,
        _spl_token_program,
        rent_program,
        fund_token_mint,
        fund_token_ref,
        vaults_assets_info,
        custodies_assets_info
        ] = accounts
    {
        // validate accounts
        if fund_authority.key != &fund.fund_authority
            || fund_token_ref.key != &fund.fund_token_ref
            || fund_program.key != &fund.fund_program_id
        {
            msg!("Error: Invalid Fund accounts");
            return Err(ProgramError::Custom(511));
        }

        // init fund authority account
        msg!("Init Fund authority");
        pda::init_system_account(
            admin_account,
            fund_authority,
            &fund.fund_program_id,
            &fund.fund_program_id,
            &[b"fund_authority", fund.name.as_bytes()],
            0,
        )?;

        // init fund info account
        msg!("Init Fund info");
        pda::init_system_account(
            admin_account,
            fund_info_account,
            &fund.fund_program_id,
            &fund.fund_program_id,
            &[b"info_account", fund.name.as_bytes()],
            FundInfo::LEN,
        )?;
        let mut fund_info = FundInfo::new(fund_info_account);
        fund_info.init(&fund.name)?;

        // init fund token mint
        msg!("Init Fund token mint");
        let fund_token = account::unpack::<Token>(fund_token_ref, "Fund Token")?;
        if fund_token_mint.key != &fund_token.mint {
            msg!("Error: Invalid Fund token mint");
            return Err(ProgramError::Custom(510));
        }
        pda::init_mint(
            admin_account,
            fund_token_mint,
            fund_authority,
            rent_program,
            &fund.fund_program_id,
            &[b"fund_token_mint", fund.name.as_bytes()],
            fund_token.decimals,
        )?;

        // init vaults assets info
        if account::is_empty(vaults_assets_info)? {
            msg!("Init vaults assets info");
            let bump = pda::init_system_account(
                admin_account,
                vaults_assets_info,
                &fund.fund_program_id,
                &fund.fund_program_id,
                &[b"vaults_assets_info", fund.name.as_bytes()],
                FundAssets::LEN,
            )?;
            let mut fund_assets = account::unpack::<FundAssets>(vaults_assets_info, "Vaults assets")?;
            fund_assets.asset_type = FundAssetType::Vault;
            fund_assets.target_hash = 0;
            fund_assets.current_hash = 0;
            fund_assets.current_cycle = 0;
            fund_assets.current_assets_usd = 0.0;
            fund_assets.cycle_start_time = 0;
            fund_assets.cycle_end_time = 0;
            fund_assets.bump = bump;
            fund_assets.pack(*vaults_assets_info.try_borrow_mut_data()?)?;
        }

        // init custodies assets info
        if account::is_empty(custodies_assets_info)? {
            msg!("Init custodies assets info");
            let bump = pda::init_system_account(
                admin_account,
                custodies_assets_info,
                &fund.fund_program_id,
                &fund.fund_program_id,
                &[b"custodies_assets_info", fund.name.as_bytes()],
                FundAssets::LEN,
            )?;
            let mut fund_assets =
                account::unpack::<FundAssets>(custodies_assets_info, "Custodies assets")?;
            fund_assets.asset_type = FundAssetType::Custody;
            fund_assets.target_hash = 0;
            fund_assets.current_hash = 0;
            fund_assets.current_cycle = 0;
            fund_assets.current_assets_usd = 0.0;
            fund_assets.cycle_start_time = 0;
            fund_assets.cycle_end_time = 0;
            fund_assets.bump = bump;
            fund_assets.pack(*custodies_assets_info.try_borrow_mut_data()?)?;
        }

        Ok(())
    } else {
        Err(ProgramError::NotEnoughAccountKeys)
    }
}
