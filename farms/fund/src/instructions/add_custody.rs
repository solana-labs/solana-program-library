//! Fund AddCustody instruction handler

use {
    crate::{common, fund_info::FundInfo},
    solana_farm_sdk::{
        fund::{Fund, FundAssetType, FundCustody, FundCustodyType, DISCRIMINATOR_FUND_CUSTODY},
        id::{main_router, zero},
        program::{account, pda},
        token::{OracleType, Token},
        traits::Packed,
    },
    solana_program::{
        account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
    },
};

pub fn add_custody(
    fund: &Fund,
    accounts: &[AccountInfo],
    target_hash: u64,
    custody_id: u32,
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
        _associated_token_program,
        rent_program,
        custodies_assets_info,
        custody_account,
        custody_fees_account,
        custody_metadata,
        custody_token_metadata,
        custody_token_mint
        ] = accounts
    {
        // validate accounts
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
        if custody_token_metadata.owner != &main_router::id() {
            msg!("Error: Invalid custody token metadata owner");
            return Err(ProgramError::IllegalOwner);
        }

        if account::exists(custody_metadata)? || account::exists(custody_account)? ||
            account::exists(custody_fees_account)? {
            msg!("Error: Custody already initialized");
            return Err(ProgramError::AccountAlreadyInitialized);
        }

        // init custody metadata account
        msg!("Init custody metadata account");
        let custody_token = account::unpack::<Token>(custody_token_metadata, "custody token")?;
        if &custody_token.mint != custody_token_mint.key {
            msg!("Error: Custody token mint mismatch");
            return Err(ProgramError::Custom(502));
        }
        let is_vault_token =
            custody_token.name.len() > 3 && ["LP.", "VT."].contains(&&custody_token.name[..3]);
        if !is_vault_token && custody_token.oracle_type == OracleType::Unsupported {
            msg!(
                "Error: Oracle is not supported for token {}",
                custody_token.name
            );
            return Err(ProgramError::InvalidAccountData);
        }
        let custody_seed_str: &[u8] = match custody_type {
            FundCustodyType::DepositWithdraw => b"fund_wd_custody_info",
            FundCustodyType::Trading => b"fund_td_custody_info",
        };
        let custody_seeds = &[
            custody_seed_str,
            custody_token.name.as_bytes(),
            fund.name.as_bytes(),
        ];
        let bump = pda::init_system_account(
            admin_account,
            custody_metadata,
            &fund.fund_program_id,
            &fund.fund_program_id,
            custody_seeds,
            FundCustody::LEN,
        )?;

        let custody = FundCustody {
            discriminator: DISCRIMINATOR_FUND_CUSTODY,
            fund_ref: *fund_metadata.key,
            custody_id,
            custody_type,
            is_vault_token,
            token_ref: *custody_token_metadata.key,
            address: *custody_account.key,
            fees_address: if is_vault_token {
                zero::id()
            } else {
                *custody_fees_account.key
            },
            bump,
        };
        custody.pack(*custody_metadata.try_borrow_mut_data()?)?;

        // init token accounts
        msg!("Init custody token account");
        if matches!(custody_type, FundCustodyType::DepositWithdraw) {
            pda::init_token_account(
                admin_account,
                custody_account,
                custody_token_mint,
                fund_authority,
                rent_program,
                &fund.fund_program_id,
                &[
                    b"fund_wd_custody_account",
                    custody_token.name.as_bytes(),
                    fund.name.as_bytes(),
                ],
            )?;
        } else {
            pda::init_associated_token_account(
                admin_account,
                fund_authority,
                custody_account,
                custody_token_mint,
                rent_program,
            )?;
        }

        // if custody is not for a vault token then it needs a second token account
        // where fees will be collected. Also, since each non-vault custody must
        // be counted in update_assets_with_custody() we reset fund_assets stats.
        if !is_vault_token {
            msg!("Init fee custody token account");
            let custody_seed_str: &[u8] = match custody_type {
                FundCustodyType::DepositWithdraw => b"fund_wd_custody_fees_account",
                FundCustodyType::Trading => b"fund_td_custody_fees_account",
            };
            pda::init_token_account(
                admin_account,
                custody_fees_account,
                custody_token_mint,
                fund_multisig_account,
                rent_program,
                &fund.fund_program_id,
                &[
                    custody_seed_str,
                    custody_token.name.as_bytes(),
                    fund.name.as_bytes(),
                ],
            )?;

            // update assets tracking account
            msg!("Update Fund assets account");
            let mut fund_assets = common::check_and_get_fund_assets_account(
                fund,
                custodies_assets_info,
                FundAssetType::Custody,
            )?;
            fund_assets.current_hash = 0;
            fund_assets.target_hash = target_hash;
            fund_assets.current_assets_usd = 0.0;
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
