//! Orca specific functions

use {
    crate::{
        error::FarmError,
        instruction::orca::{OrcaHarvest, OrcaStake, OrcaUnstake},
        math,
        pack::check_data_len,
        program::account,
    },
    arrayref::{array_ref, array_refs},
    solana_program::{
        account_info::AccountInfo,
        entrypoint::ProgramResult,
        instruction::{AccountMeta, Instruction},
        msg,
        program::{invoke, invoke_signed},
        program_error::ProgramError,
        pubkey::Pubkey,
    },
    spl_token_swap::instruction,
};

pub mod orca_swap {
    solana_program::declare_id!("9W959DqEETiGZocYWCQPaJ6sBmUzgfxXfqGeTEdp3aQP");
}

pub mod orca_stake {
    solana_program::declare_id!("82yxjeMsvaURa4MbZZ7WZZHfobirZYkH1zF8fmeGtyaQ");
}

pub const ORCA_FEE: f64 = 0.003;
pub const ORCA_FEE_NUMERATOR: u64 = 3;
pub const ORCA_FEE_DENOMINATOR: u64 = 1000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OrcaUserStakeInfo {
    pub is_initialized: u8,
    pub account_type: u8,
    pub global_farm: Pubkey,
    pub owner: Pubkey,
    pub base_tokens_converted: u64,
    pub cumulative_emissions_checkpoint: [u8; 32],
}

impl OrcaUserStakeInfo {
    pub const LEN: usize = 106;

    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        check_data_len(input, OrcaUserStakeInfo::LEN)?;

        let input = array_ref![input, 0, OrcaUserStakeInfo::LEN];

        #[allow(clippy::ptr_offset_with_cast)]
        let (
            is_initialized,
            account_type,
            global_farm,
            owner,
            base_tokens_converted,
            cumulative_emissions_checkpoint,
        ) = array_refs![input, 1, 1, 32, 32, 8, 32];

        Ok(Self {
            is_initialized: is_initialized[0],
            account_type: account_type[0],
            global_farm: Pubkey::new_from_array(*global_farm),
            owner: Pubkey::new_from_array(*owner),
            base_tokens_converted: u64::from_le_bytes(*base_tokens_converted),
            cumulative_emissions_checkpoint: *cumulative_emissions_checkpoint,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OrcaFarmState {
    pub is_initialized: u8,
    pub account_type: u8,
    pub nonce: u8,
    pub token_program: Pubkey,
    pub emissions_authority: Pubkey,
    pub remove_rewards_authority: Pubkey,
    pub base_token_mint: Pubkey,
    pub base_token_vault: Pubkey,
    pub reward_token_vault: Pubkey,
    pub farm_token_mint: Pubkey,
    pub emissions_per_sec_numerator: u64,
    pub emissions_per_sec_denominator: u64,
    pub last_updated_timestamp: u64,
    pub cumulative_emissions_per_farm_token: [u8; 32],
}

impl OrcaFarmState {
    pub const LEN: usize = 283;

    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        check_data_len(input, OrcaFarmState::LEN)?;

        let input = array_ref![input, 0, OrcaFarmState::LEN];

        #[allow(clippy::ptr_offset_with_cast)]
        let (
            is_initialized,
            account_type,
            nonce,
            token_program,
            emissions_authority,
            remove_rewards_authority,
            base_token_mint,
            base_token_vault,
            reward_token_vault,
            farm_token_mint,
            emissions_per_sec_numerator,
            emissions_per_sec_denominator,
            last_updated_timestamp,
            cumulative_emissions_per_farm_token,
        ) = array_refs![input, 1, 1, 1, 32, 32, 32, 32, 32, 32, 32, 8, 8, 8, 32];

        Ok(Self {
            is_initialized: is_initialized[0],
            account_type: account_type[0],
            nonce: nonce[0],
            token_program: Pubkey::new_from_array(*token_program),
            emissions_authority: Pubkey::new_from_array(*emissions_authority),
            remove_rewards_authority: Pubkey::new_from_array(*remove_rewards_authority),
            base_token_mint: Pubkey::new_from_array(*base_token_mint),
            base_token_vault: Pubkey::new_from_array(*base_token_vault),
            reward_token_vault: Pubkey::new_from_array(*reward_token_vault),
            farm_token_mint: Pubkey::new_from_array(*farm_token_mint),
            emissions_per_sec_numerator: u64::from_le_bytes(*emissions_per_sec_numerator),
            emissions_per_sec_denominator: u64::from_le_bytes(*emissions_per_sec_denominator),
            last_updated_timestamp: u64::from_le_bytes(*last_updated_timestamp),
            cumulative_emissions_per_farm_token: *cumulative_emissions_per_farm_token,
        })
    }
}

pub fn check_pool_program_id(program_id: &Pubkey) -> bool {
    program_id == &orca_swap::id()
}

pub fn check_stake_program_id(program_id: &Pubkey) -> bool {
    program_id == &orca_stake::id()
}

/// Returns amount of LP tokens staked as recorded in the specified stake account
pub fn get_stake_account_balance(stake_account: &AccountInfo) -> Result<u64, ProgramError> {
    let data = stake_account.try_borrow_data()?;
    Ok(OrcaUserStakeInfo::unpack(&data)?.base_tokens_converted)
}

pub fn get_pool_token_balances<'a, 'b>(
    pool_token_a_account: &'a AccountInfo<'b>,
    pool_token_b_account: &'a AccountInfo<'b>,
) -> Result<(u64, u64), ProgramError> {
    Ok((
        account::get_token_balance(pool_token_a_account)?,
        account::get_token_balance(pool_token_b_account)?,
    ))
}

pub fn get_pool_deposit_amounts<'a, 'b>(
    pool_token_a_account: &'a AccountInfo<'b>,
    pool_token_b_account: &'a AccountInfo<'b>,
    lp_token_mint: &'a AccountInfo<'b>,
    max_token_a_amount: u64,
    max_token_b_amount: u64,
) -> Result<(u64, u64, u64), ProgramError> {
    if max_token_a_amount == 0 && max_token_b_amount == 0 {
        msg!("Error: At least one of token amounts must be non-zero");
        return Err(ProgramError::InvalidArgument);
    }
    let mut token_a_amount = max_token_a_amount;
    let mut token_b_amount = max_token_b_amount;
    let (token_a_balance, token_b_balance) =
        get_pool_token_balances(pool_token_a_account, pool_token_b_account)?;

    if token_a_balance == 0 || token_b_balance == 0 {
        if max_token_a_amount == 0 || max_token_b_amount == 0 {
            msg!("Error: Both amounts must be specified for the initial deposit to an empty pool");
            return Err(ProgramError::InvalidArgument);
        } else {
            return Ok((1, max_token_a_amount, max_token_b_amount));
        }
    }

    if max_token_a_amount == 0 {
        let estimated_coin_amount = math::checked_as_u64(math::checked_div(
            math::checked_mul(token_a_balance as u128, max_token_b_amount as u128)?,
            token_b_balance as u128,
        )?)?;
        token_a_amount = if estimated_coin_amount > 1 {
            estimated_coin_amount - 1
        } else {
            0
        };
    } else if max_token_b_amount == 0 {
        token_b_amount = math::checked_add(
            math::checked_as_u64(math::checked_div(
                math::checked_mul(token_b_balance as u128, max_token_a_amount as u128)?,
                token_a_balance as u128,
            )?)?,
            1,
        )?;
    }

    let min_lp_tokens_out = estimate_lp_tokens_amount(
        lp_token_mint,
        token_a_amount,
        token_b_amount,
        token_a_balance,
        token_b_balance,
    )?;

    Ok((min_lp_tokens_out, token_a_amount, token_b_amount))
}

pub fn get_pool_withdrawal_amounts<'a, 'b>(
    pool_token_a_account: &'a AccountInfo<'b>,
    pool_token_b_account: &'a AccountInfo<'b>,
    lp_token_mint: &'a AccountInfo<'b>,
    lp_token_amount: u64,
) -> Result<(u64, u64), ProgramError> {
    if lp_token_amount == 0 {
        msg!("Error: LP token amount must be non-zero");
        return Err(ProgramError::InvalidArgument);
    }
    let (token_a_balance, token_b_balance) =
        get_pool_token_balances(pool_token_a_account, pool_token_b_account)?;
    if token_a_balance == 0 && token_b_balance == 0 {
        return Ok((0, 0));
    }
    let lp_token_supply = account::get_token_supply(lp_token_mint)?;
    if lp_token_supply == 0 {
        return Ok((0, 0));
    }
    Ok((
        math::checked_as_u64(math::checked_div(
            math::checked_mul(token_a_balance as u128, lp_token_amount as u128)?,
            lp_token_supply as u128,
        )?)?,
        math::checked_as_u64(math::checked_div(
            math::checked_mul(token_b_balance as u128, lp_token_amount as u128)?,
            lp_token_supply as u128,
        )?)?,
    ))
}

pub fn get_pool_swap_amounts<'a, 'b>(
    pool_token_a_account: &'a AccountInfo<'b>,
    pool_token_b_account: &'a AccountInfo<'b>,
    token_a_amount_in: u64,
    token_b_amount_in: u64,
) -> Result<(u64, u64), ProgramError> {
    if (token_a_amount_in == 0 && token_b_amount_in == 0)
        || (token_a_amount_in > 0 && token_b_amount_in > 0)
    {
        msg!("Error: One and only one of token amounts must be non-zero");
        return Err(ProgramError::InvalidArgument);
    }
    let (token_a_balance, token_b_balance) =
        get_pool_token_balances(pool_token_a_account, pool_token_b_account)?;
    if token_a_balance == 0 || token_b_balance == 0 {
        msg!("Error: Can't swap in an empty pool");
        return Err(FarmError::EmptyPool.into());
    }
    let token_a_balance = token_a_balance as u128;
    let token_b_balance = token_b_balance as u128;
    if token_a_amount_in == 0 {
        // b to a
        let amount_in_no_fee =
            math::get_no_fee_amount(token_b_amount_in, ORCA_FEE_NUMERATOR, ORCA_FEE_DENOMINATOR)?
                as u128;
        let estimated_token_a_amount = math::checked_as_u64(math::checked_div(
            math::checked_mul(token_a_balance, amount_in_no_fee)?,
            math::checked_add(token_b_balance, amount_in_no_fee)?,
        )?)?;

        Ok((
            token_b_amount_in,
            math::get_no_fee_amount(estimated_token_a_amount, 3, 100)?,
        ))
    } else {
        // a to b
        let amount_in_no_fee =
            math::get_no_fee_amount(token_a_amount_in, ORCA_FEE_NUMERATOR, ORCA_FEE_DENOMINATOR)?
                as u128;
        let estimated_token_b_amount = math::checked_as_u64(math::checked_div(
            math::checked_mul(token_b_balance as u128, amount_in_no_fee)?,
            math::checked_add(token_a_balance as u128, amount_in_no_fee)?,
        )?)?;

        Ok((
            token_a_amount_in,
            math::get_no_fee_amount(estimated_token_b_amount, 3, 100)?,
        ))
    }
}

pub fn estimate_lp_tokens_amount(
    lp_token_mint: &AccountInfo,
    token_a_deposit: u64,
    token_b_deposit: u64,
    pool_token_a_balance: u64,
    pool_token_b_balance: u64,
) -> Result<u64, ProgramError> {
    if pool_token_a_balance != 0 && pool_token_b_balance != 0 {
        Ok(std::cmp::min(
            math::checked_as_u64(math::checked_div(
                math::checked_mul(
                    token_a_deposit as u128,
                    account::get_token_supply(lp_token_mint)? as u128,
                )?,
                pool_token_a_balance as u128,
            )?)?,
            math::checked_as_u64(math::checked_div(
                math::checked_mul(
                    token_b_deposit as u128,
                    account::get_token_supply(lp_token_mint)? as u128,
                )?,
                pool_token_b_balance as u128,
            )?)?,
        ))
    } else if pool_token_a_balance != 0 {
        math::checked_as_u64(math::checked_div(
            math::checked_mul(
                token_a_deposit as u128,
                account::get_token_supply(lp_token_mint)? as u128,
            )?,
            pool_token_a_balance as u128,
        )?)
    } else if pool_token_b_balance != 0 {
        math::checked_as_u64(math::checked_div(
            math::checked_mul(
                token_b_deposit as u128,
                account::get_token_supply(lp_token_mint)? as u128,
            )?,
            pool_token_b_balance as u128,
        )?)
    } else {
        Ok(0)
    }
}

pub fn add_liquidity(
    accounts: &[AccountInfo],
    max_token_a_amount: u64,
    max_token_b_amount: u64,
    min_lp_token_amount: u64,
) -> ProgramResult {
    #[allow(clippy::deprecated_cfg_attr)]
    #[cfg_attr(rustfmt, rustfmt_skip)]
    if let [
        user_account,
        user_token_a_account,
        user_token_b_account,
        user_lp_token_account,
        pool_program_id,
        pool_token_a_account,
        pool_token_b_account,
        lp_token_mint,
        _spl_token_id,
        amm_id,
        amm_authority
        ] = accounts
    {
        if !check_pool_program_id(pool_program_id.key) {
            return Err(ProgramError::IncorrectProgramId);
        }

        let data = instruction::DepositAllTokenTypes {
            pool_token_amount: min_lp_token_amount,
            maximum_token_a_amount: max_token_a_amount,
            maximum_token_b_amount: max_token_b_amount,
        };

        let instruction = instruction::deposit_all_token_types(
            pool_program_id.key,
            &spl_token::id(),
            amm_id.key,
            amm_authority.key,
            user_account.key,
            user_token_a_account.key,
            user_token_b_account.key,
            pool_token_a_account.key,
            pool_token_b_account.key,
            lp_token_mint.key,
            user_lp_token_account.key,
            data,
        )?;

        invoke(&instruction, accounts)
    } else {
        Err(ProgramError::NotEnoughAccountKeys)
    }
}

pub fn add_liquidity_with_seeds(
    accounts: &[AccountInfo],
    seeds: &[&[&[u8]]],
    max_token_a_amount: u64,
    max_token_b_amount: u64,
    min_lp_token_amount: u64,
) -> ProgramResult {
    #[allow(clippy::deprecated_cfg_attr)]
    #[cfg_attr(rustfmt, rustfmt_skip)]
    if let [
        authority_account,
        token_a_custody_account,
        token_b_custody_account,
        lp_token_custody_account,
        pool_program_id,
        pool_token_a_account,
        pool_token_b_account,
        lp_token_mint,
        _spl_token_id,
        amm_id,
        amm_authority
        ] = accounts
    {
        if !check_pool_program_id(pool_program_id.key) {
            return Err(ProgramError::IncorrectProgramId);
        }

        let data = instruction::DepositAllTokenTypes {
            pool_token_amount: min_lp_token_amount,
            maximum_token_a_amount: max_token_a_amount,
            maximum_token_b_amount: max_token_b_amount,
        };

        let instruction = instruction::deposit_all_token_types(
            pool_program_id.key,
            &spl_token::id(),
            amm_id.key,
            amm_authority.key,
            authority_account.key,
            token_a_custody_account.key,
            token_b_custody_account.key,
            pool_token_a_account.key,
            pool_token_b_account.key,
            lp_token_mint.key,
            lp_token_custody_account.key,
            data,
        )?;

        invoke_signed(&instruction, accounts, seeds)
    } else {
        Err(ProgramError::NotEnoughAccountKeys)
    }
}

pub fn remove_liquidity_with_seeds(
    accounts: &[AccountInfo],
    seeds: &[&[&[u8]]],
    lp_amount: u64,
    min_token_a_amount: u64,
    min_token_b_amount: u64,
) -> ProgramResult {
    #[allow(clippy::deprecated_cfg_attr)]
    #[cfg_attr(rustfmt, rustfmt_skip)]
    if let [
        authority_account,
        token_a_custody_account,
        token_b_custody_account,
        lp_token_custody_account,
        pool_program_id,
        pool_token_a_account,
        pool_token_b_account,
        lp_token_mint,
        _spl_token_id,
        amm_id,
        amm_authority,
        fees_account
        ] = accounts
    {
        if !check_pool_program_id(pool_program_id.key) {
            return Err(ProgramError::IncorrectProgramId);
        }
        
        let data = instruction::WithdrawAllTokenTypes {
            pool_token_amount: lp_amount,
            minimum_token_a_amount: min_token_a_amount,
            minimum_token_b_amount: min_token_b_amount,
        };

        let instruction = instruction::withdraw_all_token_types(
            pool_program_id.key,
            &spl_token::id(),
            amm_id.key,
            amm_authority.key,
            authority_account.key,
            lp_token_mint.key,
            fees_account.key,
            lp_token_custody_account.key,
            pool_token_a_account.key,
            pool_token_b_account.key,
            token_a_custody_account.key,
            token_b_custody_account.key,
            data,
        )?;

        invoke_signed(&instruction, accounts, seeds)
    } else {
        Err(ProgramError::NotEnoughAccountKeys)
    }
}

pub fn stake_with_seeds(
    accounts: &[AccountInfo],
    seeds: &[&[&[u8]]],
    amount: u64,
) -> ProgramResult {
    #[allow(clippy::deprecated_cfg_attr)]
    #[cfg_attr(rustfmt, rustfmt_skip)]
    if let [
        authority_account,
        stake_info_account,
        lp_token_custody_account,
        reward_token_custody_account,
        farm_lp_token_custody_account,
        farm_lp_token_mint,
        farm_program_id,
        base_token_vault,
        reward_token_vault,
        _spl_token_id,
        farm_id,
        farm_authority
        ] = accounts
    {
        if !check_stake_program_id(farm_program_id.key) {
            return Err(ProgramError::IncorrectProgramId);
        }

        let orca_accounts = vec![
            AccountMeta::new_readonly(*authority_account.key, true),
            AccountMeta::new(*lp_token_custody_account.key, false),
            AccountMeta::new(*base_token_vault.key, false),
            AccountMeta::new_readonly(*authority_account.key, true),
            AccountMeta::new(*farm_lp_token_mint.key, false),
            AccountMeta::new(*farm_lp_token_custody_account.key, false),
            AccountMeta::new(*farm_id.key, false),
            AccountMeta::new(*stake_info_account.key, false),
            AccountMeta::new(*reward_token_vault.key, false),
            AccountMeta::new(*reward_token_custody_account.key, false),
            AccountMeta::new_readonly(*farm_authority.key, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ];

        let instruction = Instruction {
            program_id: *farm_program_id.key,
            accounts: orca_accounts,
            data: OrcaStake { amount }.to_vec()?,
        };
        
        invoke_signed(&instruction, accounts, seeds)
    } else {
        Err(ProgramError::NotEnoughAccountKeys)
    }
}

pub fn harvest_with_seeds(accounts: &[AccountInfo], seeds: &[&[&[u8]]]) -> ProgramResult {
    #[allow(clippy::deprecated_cfg_attr)]
    #[cfg_attr(rustfmt, rustfmt_skip)]
    if let [
        authority_account,
        stake_info_account,
        reward_token_custody_account,
        farm_program_id,
        base_token_vault,
        reward_token_vault,
        _spl_token_id,
        farm_id,
        farm_authority
        ] = accounts
    {
        if !check_stake_program_id(farm_program_id.key) {
            return Err(ProgramError::IncorrectProgramId);
        }
        
        let orca_accounts = vec![
            AccountMeta::new_readonly(*authority_account.key, true),
            AccountMeta::new(*farm_id.key, false),
            AccountMeta::new(*stake_info_account.key, false),
            AccountMeta::new_readonly(*base_token_vault.key, false),
            AccountMeta::new(*reward_token_vault.key, false),
            AccountMeta::new(*reward_token_custody_account.key, false),
            AccountMeta::new_readonly(*farm_authority.key, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ];

        let instruction = Instruction {
            program_id: *farm_program_id.key,
            accounts: orca_accounts,
            data: OrcaHarvest {}.to_vec()?,
        };

        invoke_signed(&instruction, accounts, seeds)
    } else {
        Err(ProgramError::NotEnoughAccountKeys)
    }
}

pub fn swap_with_seeds(
    accounts: &[AccountInfo],
    seeds: &[&[&[u8]]],
    amount_in: u64,
    min_amount_out: u64,
) -> ProgramResult {
    #[allow(clippy::deprecated_cfg_attr)]
    #[cfg_attr(rustfmt, rustfmt_skip)]
    if let [
        authority_account,
        token_a_custody_account,
        token_b_custody_account,
        pool_program_id,
        pool_token_a_account,
        pool_token_b_account,
        lp_token_mint,
        _spl_token_id,
        amm_id,
        amm_authority,
        fees_account
        ] = accounts
    {
        if !check_pool_program_id(pool_program_id.key) {
            return Err(ProgramError::IncorrectProgramId);
        }

        let data = instruction::Swap {
            amount_in,
            minimum_amount_out: min_amount_out,
        };

        let instruction = instruction::swap(
            pool_program_id.key,
            &spl_token::id(),
            amm_id.key,
            amm_authority.key,
            authority_account.key,
            token_a_custody_account.key,
            pool_token_a_account.key,
            pool_token_b_account.key,
            token_b_custody_account.key,
            lp_token_mint.key,
            fees_account.key,
            None,
            data,
        )?;

        invoke_signed(&instruction, accounts, seeds)
    } else {
        Err(ProgramError::NotEnoughAccountKeys)
    }
}

pub fn unstake_with_seeds(
    accounts: &[AccountInfo],
    seeds: &[&[&[u8]]],
    amount: u64,
) -> ProgramResult {
    #[allow(clippy::deprecated_cfg_attr)]
    #[cfg_attr(rustfmt, rustfmt_skip)]
    if let [
        authority_account,
        stake_info_account,
        lp_token_custody_account,
        reward_token_custody_account,
        farm_lp_token_custody_account,
        farm_lp_token_mint,
        farm_program_id,
        base_token_vault,
        reward_token_vault,
        _spl_token_id,
        farm_id,
        farm_authority
        ] = accounts
    {
        if !check_stake_program_id(farm_program_id.key) {
            return Err(ProgramError::IncorrectProgramId);
        }

        let orca_accounts = vec![
            AccountMeta::new_readonly(*authority_account.key, true),
            AccountMeta::new(*lp_token_custody_account.key, false),
            AccountMeta::new(*base_token_vault.key, false),
            AccountMeta::new(*farm_lp_token_mint.key, false),
            AccountMeta::new(*farm_lp_token_custody_account.key, false),
            AccountMeta::new_readonly(*authority_account.key, true),
            AccountMeta::new(*farm_id.key, false),
            AccountMeta::new(*stake_info_account.key, false),
            AccountMeta::new(*reward_token_vault.key, false),
            AccountMeta::new(*reward_token_custody_account.key, false),
            AccountMeta::new_readonly(*farm_authority.key, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ];

        let instruction = Instruction {
            program_id: *farm_program_id.key,
            accounts: orca_accounts,
            data: OrcaUnstake { amount }.to_vec()?,
        };
        invoke_signed(&instruction, accounts, seeds)
    } else {
        Err(ProgramError::NotEnoughAccountKeys)
    }
}
