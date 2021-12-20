//! Orca specific functions

use {
    crate::{math, pack::check_data_len, program::account},
    arrayref::{array_ref, array_refs},
    solana_program::{account_info::AccountInfo, msg, program_error::ProgramError, pubkey::Pubkey},
};

pub mod orca_swap {
    solana_program::declare_id!("9W959DqEETiGZocYWCQPaJ6sBmUzgfxXfqGeTEdp3aQP");
}

pub mod orca_stake {
    solana_program::declare_id!("82yxjeMsvaURa4MbZZ7WZZHfobirZYkH1zF8fmeGtyaQ");
}

pub const ORCA_FEE: f64 = 0.003;

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
        let estimated_coin_amount = math::checked_as_u64(
            token_a_balance as f64 * max_token_b_amount as f64 / (token_b_balance as f64),
        )?;
        token_a_amount = if estimated_coin_amount > 1 {
            estimated_coin_amount - 1
        } else {
            0
        };
    } else if max_token_b_amount == 0 {
        token_b_amount = math::checked_as_u64(
            token_b_balance as f64 * max_token_a_amount as f64 / (token_a_balance as f64),
        )?;
    }

    let min_lp_tokens_out = estimate_lp_tokens_amount(
        lp_token_mint,
        token_a_amount,
        token_b_amount,
        token_a_balance,
        token_b_balance,
    )?;

    Ok((
        min_lp_tokens_out,
        token_a_amount,
        math::checked_add(token_b_amount, 1)?,
    ))
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
    let stake = lp_token_amount as f64 / lp_token_supply as f64;

    Ok((
        math::checked_as_u64(token_a_balance as f64 * stake)?,
        math::checked_as_u64(token_b_balance as f64 * stake)?,
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
        return Err(ProgramError::Custom(412));
    }
    let token_a_balance = token_a_balance as f64;
    let token_b_balance = token_b_balance as f64;
    if token_a_amount_in == 0 {
        // b to a
        let amount_in_no_fee = ((token_b_amount_in as f64 * (1.0 - ORCA_FEE)) as u64) as f64;
        let estimated_token_a_amount = (token_a_balance
            - token_a_balance * token_b_balance / (token_b_balance + amount_in_no_fee))
            as u64;

        Ok((token_b_amount_in, estimated_token_a_amount))
    } else {
        // a to b
        let amount_in_no_fee = ((token_a_amount_in as f64 * (1.0 - ORCA_FEE)) as u64) as f64;
        let estimated_token_b_amount = (token_b_balance
            - token_a_balance * token_b_balance / (token_a_balance + amount_in_no_fee))
            as u64;

        Ok((token_a_amount_in, estimated_token_b_amount))
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
            math::checked_as_u64(
                (token_a_deposit as f64 / pool_token_a_balance as f64)
                    * account::get_token_supply(lp_token_mint)? as f64,
            )?,
            math::checked_as_u64(
                (token_b_deposit as f64 / pool_token_b_balance as f64)
                    * account::get_token_supply(lp_token_mint)? as f64,
            )?,
        ))
    } else if pool_token_a_balance != 0 {
        math::checked_as_u64(
            (token_a_deposit as f64 / pool_token_a_balance as f64)
                * account::get_token_supply(lp_token_mint)? as f64,
        )
    } else if pool_token_b_balance != 0 {
        math::checked_as_u64(
            (token_b_deposit as f64 / pool_token_b_balance as f64)
                * account::get_token_supply(lp_token_mint)? as f64,
        )
    } else {
        Ok(0)
    }
}
