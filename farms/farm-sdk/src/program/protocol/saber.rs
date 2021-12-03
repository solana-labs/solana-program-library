//! Saber specific functions

use {
    crate::{id::zero, pack::check_data_len, program::account},
    arrayref::{array_ref, array_refs},
    solana_program::{
        account_info::AccountInfo,
        entrypoint::ProgramResult,
        hash::Hasher,
        instruction::{AccountMeta, Instruction},
        program::{invoke, invoke_signed},
        program_error::ProgramError,
        pubkey::Pubkey,
        system_program,
    },
    stable_swap_client::instruction,
};

pub const SABER_FEE: f64 = 0.1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Miner {
    /// Key of the [Quarry] this [Miner] works on.
    pub quarry_key: Pubkey,
    /// Authority who manages this [Miner].
    /// All withdrawals of tokens must accrue to [TokenAccount]s owned by this account.
    pub authority: Pubkey,

    /// Bump.
    pub bump: u8,

    /// [TokenAccount] to hold the [Miner]'s staked LP tokens.
    pub token_vault_key: Pubkey,

    /// Stores the amount of tokens that the [Miner] may claim.
    /// Whenever the [Miner] claims tokens, this is reset to 0.
    pub rewards_earned: u64,

    /// A checkpoint of the [Quarry]'s reward tokens paid per staked token.
    ///
    /// When the [Miner] is initialized, this number starts at 0.
    /// On the first [quarry_mine::stake_tokens], the [Quarry]#update_rewards_and_miner
    /// method is called, which updates this checkpoint to the current quarry value.
    ///
    /// On a [quarry_mine::claim_rewards], the difference in checkpoints is used to calculate
    /// the amount of tokens owed.
    pub rewards_per_token_paid: u128,

    /// Number of tokens the [Miner] holds.
    pub balance: u64,

    /// Index of the [Miner].
    pub index: u64,
}

impl Miner {
    pub const LEN: usize = 145;

    pub fn get_size(&self) -> usize {
        Miner::LEN
    }

    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        check_data_len(input, Miner::LEN)?;

        let input = array_ref![input, 8, Miner::LEN - 8];
        #[allow(clippy::ptr_offset_with_cast)]
        let (
            quarry_key,
            authority,
            bump,
            token_vault_key,
            rewards_earned,
            rewards_per_token_paid,
            balance,
            index,
        ) = array_refs![input, 32, 32, 1, 32, 8, 16, 8, 8];

        Ok(Self {
            quarry_key: Pubkey::new_from_array(*quarry_key),
            authority: Pubkey::new_from_array(*authority),
            bump: bump[0],
            token_vault_key: Pubkey::new_from_array(*token_vault_key),
            rewards_earned: u64::from_le_bytes(*rewards_earned),
            rewards_per_token_paid: u128::from_le_bytes(*rewards_per_token_paid),
            balance: u64::from_le_bytes(*balance),
            index: u64::from_le_bytes(*index),
        })
    }
}

/// Returns amount of LP tokens staked as recorded in the specified stake account
pub fn get_stake_account_balance(stake_account: &AccountInfo) -> Result<u64, ProgramError> {
    let data = stake_account.try_borrow_data()?;
    Ok(Miner::unpack(&data)?.balance)
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

#[allow(clippy::too_many_arguments)]
pub fn wrap_token<'a, 'b>(
    wrapper: &'a AccountInfo<'b>,
    wrapped_token_mint: &'a AccountInfo<'b>,
    wrapper_vault: &'a AccountInfo<'b>,
    owner: &'a AccountInfo<'b>,
    underlying_token_account: &'a AccountInfo<'b>,
    wrapped_token_account: &'a AccountInfo<'b>,
    decimal_wrapper_program: &Pubkey,
    amount: u64,
) -> ProgramResult {
    decimal_wrapper_invoke(
        wrapper,
        wrapped_token_mint,
        wrapper_vault,
        owner,
        underlying_token_account,
        wrapped_token_account,
        decimal_wrapper_program,
        "global:deposit",
        &[&[&[]]],
        amount,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn unwrap_token<'a, 'b>(
    wrapper: &'a AccountInfo<'b>,
    wrapped_token_mint: &'a AccountInfo<'b>,
    wrapper_vault: &'a AccountInfo<'b>,
    owner: &'a AccountInfo<'b>,
    underlying_token_account: &'a AccountInfo<'b>,
    wrapped_token_account: &'a AccountInfo<'b>,
    decimal_wrapper_program: &Pubkey,
    amount: u64,
) -> ProgramResult {
    decimal_wrapper_invoke(
        wrapper,
        wrapped_token_mint,
        wrapper_vault,
        owner,
        underlying_token_account,
        wrapped_token_account,
        decimal_wrapper_program,
        "global:withdraw",
        &[&[&[]]],
        amount,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn wrap_token_with_seeds<'a, 'b>(
    wrapper: &'a AccountInfo<'b>,
    wrapped_token_mint: &'a AccountInfo<'b>,
    wrapper_vault: &'a AccountInfo<'b>,
    authority: &'a AccountInfo<'b>,
    underlying_token_account: &'a AccountInfo<'b>,
    wrapped_token_account: &'a AccountInfo<'b>,
    decimal_wrapper_program: &Pubkey,
    seeds: &[&[&[u8]]],
    amount: u64,
) -> ProgramResult {
    decimal_wrapper_invoke(
        wrapper,
        wrapped_token_mint,
        wrapper_vault,
        authority,
        underlying_token_account,
        wrapped_token_account,
        decimal_wrapper_program,
        "global:deposit",
        seeds,
        amount,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn unwrap_token_with_seeds<'a, 'b>(
    wrapper: &'a AccountInfo<'b>,
    wrapped_token_mint: &'a AccountInfo<'b>,
    wrapper_vault: &'a AccountInfo<'b>,
    authority: &'a AccountInfo<'b>,
    underlying_token_account: &'a AccountInfo<'b>,
    wrapped_token_account: &'a AccountInfo<'b>,
    decimal_wrapper_program: &Pubkey,
    seeds: &[&[&[u8]]],
    amount: u64,
) -> ProgramResult {
    decimal_wrapper_invoke(
        wrapper,
        wrapped_token_mint,
        wrapper_vault,
        authority,
        underlying_token_account,
        wrapped_token_account,
        decimal_wrapper_program,
        "global:withdraw",
        seeds,
        amount,
    )
}

#[allow(clippy::too_many_arguments)]
fn decimal_wrapper_invoke<'a, 'b>(
    wrapper: &'a AccountInfo<'b>,
    wrapped_token_mint: &'a AccountInfo<'b>,
    wrapper_vault: &'a AccountInfo<'b>,
    owner: &'a AccountInfo<'b>,
    underlying_token_account: &'a AccountInfo<'b>,
    wrapped_token_account: &'a AccountInfo<'b>,
    decimal_wrapper_program: &Pubkey,
    instruction: &str,
    seeds: &[&[&[u8]]],
    amount: u64,
) -> ProgramResult {
    let mut hasher = Hasher::default();
    hasher.hash(instruction.as_bytes());

    let mut data = hasher.result().as_ref()[..8].to_vec();
    data.extend_from_slice(&amount.to_le_bytes());

    let accounts = vec![
        AccountMeta::new_readonly(*wrapper.key, false),
        AccountMeta::new(*wrapped_token_mint.key, false),
        AccountMeta::new(*wrapper_vault.key, false),
        AccountMeta::new_readonly(*owner.key, true),
        AccountMeta::new(*underlying_token_account.key, false),
        AccountMeta::new(*wrapped_token_account.key, false),
        AccountMeta::new_readonly(spl_token::id(), false),
    ];

    if seeds[0][0].is_empty() {
        invoke(
            &Instruction {
                program_id: *decimal_wrapper_program,
                data,
                accounts,
            },
            &[
                wrapper.clone(),
                wrapped_token_mint.clone(),
                wrapper_vault.clone(),
                owner.clone(),
                underlying_token_account.clone(),
                wrapped_token_account.clone(),
            ],
        )
    } else {
        invoke_signed(
            &Instruction {
                program_id: *decimal_wrapper_program,
                data,
                accounts,
            },
            &[
                wrapper.clone(),
                wrapped_token_mint.clone(),
                wrapper_vault.clone(),
                owner.clone(),
                underlying_token_account.clone(),
                wrapped_token_account.clone(),
            ],
            seeds,
        )
    }
}

pub fn user_init_with_seeds(accounts: &[AccountInfo], seeds: &[&[&[u8]]]) -> ProgramResult {
    if let [authority_account, funding_account, farm_program_id, lp_token_mint, miner, miner_vault, quarry, rewarder] =
        accounts
    {
        if &quarry_mine::id() != farm_program_id.key {
            return Err(ProgramError::IncorrectProgramId);
        }

        let (miner_derived, bump) = Pubkey::find_program_address(
            &[
                b"Miner",
                &quarry.key.to_bytes(),
                &authority_account.key.to_bytes(),
            ],
            &quarry_mine::id(),
        );

        if &miner_derived != miner.key {
            return Err(ProgramError::InvalidSeeds);
        }

        let mut hasher = Hasher::default();
        hasher.hash(b"global:create_miner");

        let mut data = hasher.result().as_ref()[..8].to_vec();
        data.push(bump);

        let saber_accounts = vec![
            AccountMeta::new(*authority_account.key, true),
            AccountMeta::new(*miner.key, false),
            AccountMeta::new(*quarry.key, false),
            AccountMeta::new(*rewarder.key, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new(*funding_account.key, true),
            AccountMeta::new(*lp_token_mint.key, false),
            AccountMeta::new(*miner_vault.key, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ];

        let instruction = Instruction {
            program_id: quarry_mine::id(),
            accounts: saber_accounts,
            data,
        };

        invoke_signed(&instruction, accounts, seeds)
    } else {
        Err(ProgramError::NotEnoughAccountKeys)
    }
}

pub fn add_liquidity(
    accounts: &[AccountInfo],
    max_token_a_amount: u64,
    max_token_b_amount: u64,
) -> ProgramResult {
    if let [user_account, user_token_a_account, user_token_b_account, user_lp_token_account, pool_program_id, pool_token_a_account, pool_token_b_account, lp_token_mint, _spl_token_id, _clock_id, swap_account, swap_authority] =
        accounts
    {
        if &stable_swap_client::id() != pool_program_id.key {
            return Err(ProgramError::IncorrectProgramId);
        }

        let instruction = instruction::deposit(
            &spl_token::id(),
            swap_account.key,
            swap_authority.key,
            user_account.key,
            user_token_a_account.key,
            user_token_b_account.key,
            pool_token_a_account.key,
            pool_token_b_account.key,
            lp_token_mint.key,
            user_lp_token_account.key,
            max_token_a_amount,
            max_token_b_amount,
            1,
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
) -> ProgramResult {
    if let [authority_account, token_a_custody_account, token_b_custody_account, lp_token_custody_account, pool_program_id, pool_token_a_account, pool_token_b_account, lp_token_mint, _spl_token_id, _clock_id, swap_account, swap_authority] =
        accounts
    {
        if &stable_swap_client::id() != pool_program_id.key {
            return Err(ProgramError::IncorrectProgramId);
        }

        let instruction = instruction::deposit(
            &spl_token::id(),
            swap_account.key,
            swap_authority.key,
            authority_account.key,
            token_a_custody_account.key,
            token_b_custody_account.key,
            pool_token_a_account.key,
            pool_token_b_account.key,
            lp_token_mint.key,
            lp_token_custody_account.key,
            max_token_a_amount,
            max_token_b_amount,
            1,
        )?;

        invoke_signed(&instruction, accounts, seeds)
    } else {
        Err(ProgramError::NotEnoughAccountKeys)
    }
}

pub fn remove_liquidity_with_seeds(
    accounts: &[AccountInfo],
    seeds: &[&[&[u8]]],
    amount: u64,
) -> ProgramResult {
    if let [authority_account, token_a_custody_account, token_b_custody_account, lp_token_custody_account, pool_program_id, pool_token_a_account, pool_token_b_account, lp_token_mint, _spl_token_id, swap_account, swap_authority, fees_account_a, fees_account_b] =
        accounts
    {
        if &stable_swap_client::id() != pool_program_id.key {
            return Err(ProgramError::IncorrectProgramId);
        }

        let instruction = instruction::withdraw(
            &spl_token::id(),
            swap_account.key,
            swap_authority.key,
            authority_account.key,
            lp_token_mint.key,
            lp_token_custody_account.key,
            pool_token_a_account.key,
            pool_token_b_account.key,
            token_a_custody_account.key,
            token_b_custody_account.key,
            fees_account_a.key,
            fees_account_b.key,
            amount,
            1,
            1,
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
    if let [authority_account, lp_token_custody_account, farm_program_id, _spl_token_id, miner, miner_vault, quarry, rewarder] =
        accounts
    {
        if &quarry_mine::id() != farm_program_id.key {
            return Err(ProgramError::IncorrectProgramId);
        }

        let mut hasher = Hasher::default();
        hasher.hash(b"global:stake_tokens");

        let mut data = hasher.result().as_ref()[..8].to_vec();
        data.extend_from_slice(&amount.to_le_bytes());

        let saber_accounts = vec![
            AccountMeta::new_readonly(*authority_account.key, true),
            AccountMeta::new(*miner.key, false),
            AccountMeta::new(*quarry.key, false),
            AccountMeta::new(*miner_vault.key, false),
            AccountMeta::new(*lp_token_custody_account.key, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(*rewarder.key, false),
        ];

        let instruction = Instruction {
            program_id: quarry_mine::id(),
            accounts: saber_accounts,
            data,
        };

        invoke_signed(&instruction, accounts, seeds)
    } else {
        Err(ProgramError::NotEnoughAccountKeys)
    }
}

pub fn claim_rewards_with_seeds(accounts: &[AccountInfo], seeds: &[&[&[u8]]]) -> ProgramResult {
    if let [authority_account, iou_token_custody_account, farm_program_id, _spl_token_id, _zero_id, miner, rewarder, minter, mint_wrapper, mint_wrapper_program, iou_token_mint, iou_fees_account, quarry] =
        accounts
    {
        if &quarry_mine::id() != farm_program_id.key
            || &quarry_mint_wrapper::id() != mint_wrapper_program.key
        {
            return Err(ProgramError::IncorrectProgramId);
        }

        // harvest IOU rewards
        let mut hasher = Hasher::default();
        hasher.hash(b"global:claim_rewards");

        let data = hasher.result().as_ref()[..8].to_vec();

        let saber_accounts = vec![
            AccountMeta::new(*mint_wrapper.key, false),
            AccountMeta::new_readonly(*mint_wrapper_program.key, false),
            AccountMeta::new(*minter.key, false),
            AccountMeta::new(*iou_token_mint.key, false),
            AccountMeta::new(*iou_token_custody_account.key, false),
            AccountMeta::new(*iou_fees_account.key, false),
            AccountMeta::new_readonly(*authority_account.key, true),
            AccountMeta::new(*miner.key, false),
            AccountMeta::new(*quarry.key, false),
            AccountMeta::new(zero::id(), false),
            AccountMeta::new(zero::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(*rewarder.key, false),
        ];

        let instruction = Instruction {
            program_id: quarry_mine::id(),
            accounts: saber_accounts,
            data,
        };

        invoke_signed(&instruction, accounts, seeds)
    } else {
        Err(ProgramError::NotEnoughAccountKeys)
    }
}

pub fn redeem_rewards_with_seeds(accounts: &[AccountInfo], seeds: &[&[&[u8]]]) -> ProgramResult {
    if let [authority_account, iou_token_custody_account, sbr_token_custody_account, _spl_token_id, redeemer, redeemer_program, sbr_token_mint, iou_token_mint, saber_vault, saber_mint_proxy_program, mint_proxy_authority, mint_proxy_state, minter_info] =
        accounts
    {
        // convert IOU to Saber
        let mut hasher = Hasher::default();
        hasher.hash(b"global:redeem_all_tokens_from_mint_proxy");

        let data = hasher.result().as_ref()[..8].to_vec();

        let saber_accounts = vec![
            AccountMeta::new_readonly(*redeemer.key, false),
            AccountMeta::new(*iou_token_mint.key, false),
            AccountMeta::new(*sbr_token_mint.key, false),
            AccountMeta::new(*saber_vault.key, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(*authority_account.key, true),
            AccountMeta::new(*iou_token_custody_account.key, false),
            AccountMeta::new(*sbr_token_custody_account.key, false),
            AccountMeta::new_readonly(*mint_proxy_authority.key, false),
            AccountMeta::new_readonly(*mint_proxy_state.key, false),
            AccountMeta::new_readonly(*saber_mint_proxy_program.key, false),
            AccountMeta::new(*minter_info.key, false),
        ];

        let instruction = Instruction {
            program_id: *redeemer_program.key,
            accounts: saber_accounts,
            data,
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
    if let [authority_account, token_a_custody_account, token_b_custody_account, pool_program_id, pool_token_a_account, pool_token_b_account, _spl_token_id, _clock_id, swap_account, swap_authority, fees_account] =
        accounts
    {
        if &stable_swap_client::id() != pool_program_id.key {
            return Err(ProgramError::IncorrectProgramId);
        }

        let instruction = instruction::swap(
            &spl_token::id(),
            swap_account.key,
            swap_authority.key,
            authority_account.key,
            token_a_custody_account.key,
            pool_token_a_account.key,
            pool_token_b_account.key,
            token_b_custody_account.key,
            fees_account.key,
            amount_in,
            min_amount_out,
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
    if let [authority_account, lp_token_custody_account, farm_program_id, _spl_token_id, miner, miner_vault, quarry, rewarder] =
        accounts
    {
        if &quarry_mine::id() != farm_program_id.key {
            return Err(ProgramError::IncorrectProgramId);
        }

        let mut hasher = Hasher::default();
        hasher.hash(b"global:withdraw_tokens");

        let mut data = hasher.result().as_ref()[..8].to_vec();
        data.extend_from_slice(&amount.to_le_bytes());

        let saber_accounts = vec![
            AccountMeta::new_readonly(*authority_account.key, true),
            AccountMeta::new(*miner.key, false),
            AccountMeta::new(*quarry.key, false),
            AccountMeta::new(*miner_vault.key, false),
            AccountMeta::new(*lp_token_custody_account.key, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(*rewarder.key, false),
        ];

        let instruction = Instruction {
            program_id: quarry_mine::id(),
            accounts: saber_accounts,
            data,
        };

        invoke_signed(&instruction, accounts, seeds)
    } else {
        Err(ProgramError::NotEnoughAccountKeys)
    }
}
