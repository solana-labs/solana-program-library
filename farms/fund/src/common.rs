//! Common functions

use {
    crate::{fund_info::FundInfo, user_info::UserInfo},
    solana_farm_sdk::{
        fund::{
            Fund, FundAssetType, FundAssets, FundCustody, FundCustodyType, FundUserRequests,
            FundVault, FundVaultType, DISCRIMINATOR_FUND_CUSTODY, DISCRIMINATOR_FUND_VAULT,
        },
        id::{main_router, zero},
        math,
        program::{account, clock},
        token::Token,
        traits::Packed,
    },
    solana_program::{
        account_info::AccountInfo, clock::UnixTimestamp, entrypoint::ProgramResult, msg,
        program_error::ProgramError, pubkey::Pubkey,
    },
};

#[allow(clippy::too_many_arguments)]
pub fn check_wd_custody_accounts<'a, 'b>(
    fund_program_id: &Pubkey,
    fund_metadata: &Pubkey,
    custody_token: &Token,
    custody_token_metadata: &'a AccountInfo<'b>,
    user_wd_token_account: &'a AccountInfo<'b>,
    custody_account: &'a AccountInfo<'b>,
    custody_fees_account: &'a AccountInfo<'b>,
    custody_metadata: &'a AccountInfo<'b>,
    oracle_account: &'a AccountInfo<'b>,
) -> ProgramResult {
    let deposit_token_mint =
        if let Ok(mint) = account::get_token_account_mint(user_wd_token_account) {
            mint
        } else {
            msg!("Error: Invalid user's deposit token account");
            return Err(ProgramError::Custom(500));
        };

    let custody_account_mint = if let Ok(mint) = account::get_token_account_mint(custody_account) {
        mint
    } else {
        msg!("Error: Invalid custody token account mint");
        return Err(ProgramError::Custom(501));
    };

    if custody_token.mint != custody_account_mint || deposit_token_mint != custody_account_mint {
        msg!("Error: Custody token mint mismatch");
        return Err(ProgramError::Custom(502));
    }

    let custody = account::unpack::<FundCustody>(custody_metadata, "custody")?;

    if &custody.token_ref != custody_token_metadata.key
        || custody_token_metadata.owner != &main_router::id()
    {
        msg!("Error: Invalid custody token account");
        return Err(ProgramError::Custom(503));
    }

    if custody_metadata.owner != fund_program_id
        || custody.discriminator != DISCRIMINATOR_FUND_CUSTODY
        || &custody.fund_ref != fund_metadata
        || custody.custody_type != FundCustodyType::DepositWithdraw
        || &custody.address != custody_account.key
        || &custody.fees_address != custody_fees_account.key
        || &custody_token.oracle_account.unwrap_or_else(zero::id) != oracle_account.key
    {
        msg!("Error: Invalid custody accounts");
        Err(ProgramError::Custom(504))
    } else {
        Ok(())
    }
}

#[allow(clippy::too_many_arguments)]
pub fn check_custody_account<'a, 'b>(
    fund_program_id: &Pubkey,
    fund_metadata: &Pubkey,
    custody_token: &Token,
    custody_token_metadata: &'a AccountInfo<'b>,
    custody_metadata: &'a AccountInfo<'b>,
    custody_type: FundCustodyType,
    custody_account: &'a AccountInfo<'b>,
    custody_fees_account: Option<&Pubkey>,
) -> ProgramResult {
    let custody_account_mint = if let Ok(mint) = account::get_token_account_mint(custody_account) {
        mint
    } else {
        msg!("Error: Invalid custody token account mint");
        return Err(ProgramError::Custom(501));
    };

    if custody_token.mint != custody_account_mint {
        msg!("Error: Custody token mint mismatch");
        return Err(ProgramError::Custom(502));
    }

    let custody = account::unpack::<FundCustody>(custody_metadata, "custody")?;

    if &custody.token_ref != custody_token_metadata.key
        || custody_token_metadata.owner != &main_router::id()
    {
        msg!("Error: Invalid custody token account");
        return Err(ProgramError::Custom(503));
    }

    if custody_metadata.owner != fund_program_id
        || custody.discriminator != DISCRIMINATOR_FUND_CUSTODY
        || &custody.fund_ref != fund_metadata
        || custody.custody_type != custody_type
        || &custody.address != custody_account.key
        || &custody.fees_address != custody_fees_account.unwrap_or(&custody.fees_address)
    {
        msg!("Error: Invalid custody accounts");
        return Err(ProgramError::Custom(504));
    }

    Ok(())
}

pub fn check_and_get_fund_assets_account(
    fund: &Fund,
    fund_assets_account: &AccountInfo,
    assets_type: FundAssetType,
) -> Result<FundAssets, ProgramError> {
    let fund_assets = account::unpack::<FundAssets>(fund_assets_account, "Fund assets")?;

    let fund_assets_info_derived = Pubkey::create_program_address(
        &[
            if assets_type == FundAssetType::Custody {
                b"custodies_assets_info"
            } else {
                b"vaults_assets_info"
            },
            fund.name.as_bytes(),
            &[fund_assets.bump],
        ],
        &fund.fund_program_id,
    )?;

    if &fund_assets_info_derived != fund_assets_account.key {
        msg!("Error: Invalid fund assets account");
        return Err(ProgramError::Custom(505));
    }

    Ok(fund_assets)
}

pub fn check_vault_account<'a, 'b>(
    fund_program_id: &Pubkey,
    fund_metadata: &'a AccountInfo<'b>,
    vault_metadata: &'a AccountInfo<'b>,
    vault_type: FundVaultType,
) -> ProgramResult {
    if vault_metadata.owner != fund_program_id {
        msg!("Error: Invalid custody owner");
        return Err(ProgramError::IllegalOwner);
    }

    let vault = account::unpack::<FundVault>(vault_metadata, "Vault")?;

    if vault.discriminator != DISCRIMINATOR_FUND_VAULT
        || vault.fund_ref != *fund_metadata.key
        || vault_type != vault.vault_type
    {
        msg!("Error: Invalid vault metadata account");
        return Err(ProgramError::Custom(506));
    }

    Ok(())
}

pub fn check_unpack_target_vault<'a, 'b>(
    fund_program_id: &Pubkey,
    router_program_id: &Pubkey,
    fund_metadata: &Pubkey,
    underlying_pool_id: &Pubkey,
    fund_vault_metadata: &'a AccountInfo<'b>,
) -> Result<FundVault, ProgramError> {
    if fund_vault_metadata.owner != fund_program_id {
        msg!("Error: Invalid Fund Vault metadata owner");
        return Err(ProgramError::IllegalOwner);
    }

    let fund_vault = account::unpack::<FundVault>(fund_vault_metadata, "Fund Vault")?;

    if &fund_vault.fund_ref != fund_metadata {
        msg!("Error: Specified Vault doesn't belong to this Fund");
        return Err(ProgramError::Custom(507));
    }

    if &fund_vault.router_program_id != router_program_id
        || &fund_vault.underlying_pool_id != underlying_pool_id
    {
        msg!("Error: Invalid target Vault");
        return Err(ProgramError::Custom(508));
    }

    Ok(fund_vault)
}

pub fn increase_vault_balance(
    fund_vault_metadata: &AccountInfo,
    vault: &FundVault,
    lp_balance_increase: u64,
) -> ProgramResult {
    if lp_balance_increase == 0 {
        return Ok(());
    }

    let updated_lp_balance = math::checked_add(vault.lp_balance, lp_balance_increase)?;
    let vault_new = FundVault {
        lp_balance: updated_lp_balance,
        balance_update_time: clock::get_time()?,
        ..*vault
    };
    vault_new.pack(*fund_vault_metadata.try_borrow_mut_data()?)?;

    Ok(())
}

pub fn decrease_vault_balance(
    fund_vault_metadata: &AccountInfo,
    vault: &FundVault,
    lp_balance_decrease: u64,
) -> ProgramResult {
    if lp_balance_decrease == 0 {
        return Ok(());
    }

    let updated_lp_balance = math::checked_sub(vault.lp_balance, lp_balance_decrease)?;
    let vault_new = FundVault {
        lp_balance: updated_lp_balance,
        balance_update_time: clock::get_time()?,
        ..*vault
    };
    vault_new.pack(*fund_vault_metadata.try_borrow_mut_data()?)?;

    Ok(())
}

pub fn check_user_requests_account<'a, 'b>(
    fund: &Fund,
    custody_token: &Token,
    user_requests: &FundUserRequests,
    user_account: &'a AccountInfo<'b>,
    user_requests_account: &'a AccountInfo<'b>,
) -> ProgramResult {
    let user_requests_derived = Pubkey::create_program_address(
        &[
            b"user_requests_account",
            custody_token.name.as_bytes(),
            user_account.key.as_ref(),
            fund.name.as_bytes(),
            &[user_requests.bump],
        ],
        &fund.fund_program_id,
    )?;

    if user_requests_account.key != &user_requests_derived {
        msg!("Error: Invalid user requests address");
        Err(ProgramError::Custom(509))
    } else {
        Ok(())
    }
}

pub fn check_fund_token_mint(fund: &Fund, fund_token_mint: &AccountInfo) -> ProgramResult {
    let fund_token_mint_derived = Pubkey::create_program_address(
        &[
            b"fund_token_mint",
            fund.name.as_bytes(),
            &[fund.fund_token_bump],
        ],
        &fund.fund_program_id,
    )?;

    if fund_token_mint.key != &fund_token_mint_derived {
        msg!("Error: Invalid Fund token mint");
        Err(ProgramError::Custom(510))
    } else {
        Ok(())
    }
}

pub fn check_assets_update_time(
    assets_update_time: UnixTimestamp,
    max_update_age_sec: u64,
) -> ProgramResult {
    let last_update_age_sec = math::checked_sub(clock::get_time()?, assets_update_time)?;
    if last_update_age_sec > max_update_age_sec as i64 {
        msg!("Error: Assets balance is stale. Contact Fund administrator.");
        Err(ProgramError::Custom(222))
    } else {
        Ok(())
    }
}

pub fn check_assets_limit_usd(
    fund_info: &FundInfo,
    deposit_value_usd: f64,
) -> Result<(), ProgramError> {
    let current_assets_usd = fund_info.get_current_assets_usd()?;
    let assets_limit = fund_info.get_assets_limit_usd()?;

    if assets_limit > 0.0 && assets_limit < deposit_value_usd + current_assets_usd {
        let amount_left = if current_assets_usd < assets_limit {
            assets_limit - current_assets_usd
        } else {
            0.0
        };
        msg!(
            "Error: Fund assets limit reached ({}). Allowed max desposit USD: {}",
            assets_limit,
            amount_left
        );
        return Err(ProgramError::Custom(223));
    }

    Ok(())
}

pub fn get_fund_token_to_mint_amount(
    current_assets_usd: f64,
    deposit_amount: u64,
    deposit_value_usd: f64,
    ft_supply_amount: u64,
) -> Result<u64, ProgramError> {
    let ft_to_mint = if ft_supply_amount == 0 {
        deposit_amount
    } else if current_assets_usd <= 0.0001 {
        msg!("Error: Assets balance is stale. Contact Fund administrator.");
        return Err(ProgramError::Custom(222));
    } else {
        math::checked_as_u64(
            math::checked_mul(
                math::checked_as_u128(deposit_value_usd / current_assets_usd * 1000000000.0)?,
                ft_supply_amount as u128,
            )? / 1000000000u128,
        )?
    };

    Ok(ft_to_mint)
}

pub fn get_fund_token_balance(
    fund_token_account: &AccountInfo,
    user_info: &UserInfo,
) -> Result<u64, ProgramError> {
    math::checked_add(
        account::get_token_balance(fund_token_account)?,
        user_info.get_virtual_tokens_balance()?,
    )
}

pub fn get_fund_token_supply(
    fund_token_mint: &AccountInfo,
    fund_info: &FundInfo,
) -> Result<u64, ProgramError> {
    math::checked_add(
        account::get_token_supply(fund_token_mint)?,
        fund_info.get_virtual_tokens_supply()?,
    )
}
