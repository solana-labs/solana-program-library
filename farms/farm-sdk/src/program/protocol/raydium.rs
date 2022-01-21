//! Raydium specific functions

use {
    crate::{
        id::zero,
        instruction::raydium::{
            RaydiumAddLiquidity, RaydiumRemoveLiquidity, RaydiumStake, RaydiumSwap, RaydiumUnstake,
        },
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
};

pub mod raydium_v2 {
    solana_program::declare_id!("RVKd61ztZW9GUwhRbbLoYVRE5Xf1B2tVscKqwZqXgEr");
}
pub mod raydium_v3 {
    solana_program::declare_id!("27haf8L6oxUeXrHrgEgsexjSY5hbVUWEmvv9Nyxg8vQv");
}
pub mod raydium_v4 {
    solana_program::declare_id!("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8");
}

pub mod raydium_stake {
    solana_program::declare_id!("EhhTKczWMGQt46ynNeRX1WfeagwwJd7ufHvCDjRxjo5Q");
}
pub mod raydium_stake_v4 {
    solana_program::declare_id!("CBuCnLe26faBpcBP2fktp4rp8abpcAnTWft6ZrP5Q4T");
}
pub mod raydium_stake_v5 {
    solana_program::declare_id!("9KEPoZmtHUrBbhWN1v1KWLMkkvwY6WLtAVUCPRtRjP4z");
}

pub const RAYDIUM_FEE: f64 = 0.0025;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RaydiumUserStakeInfo {
    pub state: u64,
    pub farm_id: Pubkey,
    pub stake_owner: Pubkey,
    pub deposit_balance: u64,
    pub reward_debt: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RaydiumUserStakeInfoV4 {
    pub state: u64,
    pub farm_id: Pubkey,
    pub stake_owner: Pubkey,
    pub deposit_balance: u64,
    pub reward_debt: u64,
    pub reward_debt_b: u64,
}

impl RaydiumUserStakeInfo {
    pub const LEN: usize = 88;

    pub fn get_size(&self) -> usize {
        RaydiumUserStakeInfo::LEN
    }

    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        check_data_len(input, RaydiumUserStakeInfo::LEN)?;

        let input = array_ref![input, 0, RaydiumUserStakeInfo::LEN];
        #[allow(clippy::ptr_offset_with_cast)]
        let (state, farm_id, stake_owner, deposit_balance, reward_debt) =
            array_refs![input, 8, 32, 32, 8, 8];

        Ok(Self {
            state: u64::from_le_bytes(*state),
            farm_id: Pubkey::new_from_array(*farm_id),
            stake_owner: Pubkey::new_from_array(*stake_owner),
            deposit_balance: u64::from_le_bytes(*deposit_balance),
            reward_debt: u64::from_le_bytes(*reward_debt),
        })
    }
}

impl RaydiumUserStakeInfoV4 {
    pub const LEN: usize = 96;

    pub fn get_size(&self) -> usize {
        RaydiumUserStakeInfoV4::LEN
    }

    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        check_data_len(input, RaydiumUserStakeInfoV4::LEN)?;

        let input = array_ref![input, 0, RaydiumUserStakeInfoV4::LEN];
        #[allow(clippy::ptr_offset_with_cast)]
        let (state, farm_id, stake_owner, deposit_balance, reward_debt, reward_debt_b) =
            array_refs![input, 8, 32, 32, 8, 8, 8];

        Ok(Self {
            state: u64::from_le_bytes(*state),
            farm_id: Pubkey::new_from_array(*farm_id),
            stake_owner: Pubkey::new_from_array(*stake_owner),
            deposit_balance: u64::from_le_bytes(*deposit_balance),
            reward_debt: u64::from_le_bytes(*reward_debt),
            reward_debt_b: u64::from_le_bytes(*reward_debt_b),
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AmmInfoV4 {
    pub status: u64,
    pub nonce: u64,
    pub order_num: u64,
    pub depth: u64,
    pub coin_decimals: u64,
    pub pc_decimals: u64,
    pub state: u64,
    pub reset_flag: u64,
    pub min_size: u64,
    pub vol_max_cut_ratio: u64,
    pub amount_wave: u64,
    pub coin_lot_size: u64,
    pub pc_lot_size: u64,
    pub min_price_multiplier: u64,
    pub max_price_multiplier: u64,
    pub sys_decimal_value: u64,
    pub min_separate_numerator: u64,
    pub min_separate_denominator: u64,
    pub trade_fee_numerator: u64,
    pub trade_fee_denominator: u64,
    pub pnl_numerator: u64,
    pub pnl_denominator: u64,
    pub swap_fee_numerator: u64,
    pub swap_fee_denominator: u64,
    pub need_take_pnl_coin: u64,
    pub need_take_pnl_pc: u64,
    pub total_pnl_pc: u64,
    pub total_pnl_coin: u64,
    pub pool_total_deposit_pc: u128,
    pub pool_total_deposit_coin: u128,
    pub swap_coin_in_amount: u128,
    pub swap_pc_out_amount: u128,
    pub swap_coin_to_pc_fee: u64,
    pub swap_pc_in_amount: u128,
    pub swap_coin_out_amount: u128,
    pub swap_pc_to_coin_fee: u64,
    pub token_coin: Pubkey,
    pub token_pc: Pubkey,
    pub coin_mint: Pubkey,
    pub pc_mint: Pubkey,
    pub lp_mint: Pubkey,
    pub open_orders: Pubkey,
    pub market: Pubkey,
    pub serum_dex: Pubkey,
    pub target_orders: Pubkey,
    pub withdraw_queue: Pubkey,
    pub token_temp_lp: Pubkey,
    pub amm_owner: Pubkey,
    pub pnl_owner: Pubkey,
}

impl AmmInfoV4 {
    pub const LEN: usize = 752;

    pub fn get_size(&self) -> usize {
        AmmInfoV4::LEN
    }

    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        check_data_len(input, AmmInfoV4::LEN)?;

        let input = array_ref![input, 0, AmmInfoV4::LEN];
        #[allow(clippy::ptr_offset_with_cast)]
        let (
            status,
            nonce,
            order_num,
            depth,
            coin_decimals,
            pc_decimals,
            state,
            reset_flag,
            min_size,
            vol_max_cut_ratio,
            amount_wave,
            coin_lot_size,
            pc_lot_size,
            min_price_multiplier,
            max_price_multiplier,
            sys_decimal_value,
            min_separate_numerator,
            min_separate_denominator,
            trade_fee_numerator,
            trade_fee_denominator,
            pnl_numerator,
            pnl_denominator,
            swap_fee_numerator,
            swap_fee_denominator,
            need_take_pnl_coin,
            need_take_pnl_pc,
            total_pnl_pc,
            total_pnl_coin,
            pool_total_deposit_pc,
            pool_total_deposit_coin,
            swap_coin_in_amount,
            swap_pc_out_amount,
            swap_coin_to_pc_fee,
            swap_pc_in_amount,
            swap_coin_out_amount,
            swap_pc_to_coin_fee,
            token_coin,
            token_pc,
            coin_mint,
            pc_mint,
            lp_mint,
            open_orders,
            market,
            serum_dex,
            target_orders,
            withdraw_queue,
            token_temp_lp,
            amm_owner,
            pnl_owner,
        ) = array_refs![
            input, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
            8, 16, 16, 16, 16, 8, 16, 16, 8, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32
        ];

        Ok(Self {
            status: u64::from_le_bytes(*status),
            nonce: u64::from_le_bytes(*nonce),
            order_num: u64::from_le_bytes(*order_num),
            depth: u64::from_le_bytes(*depth),
            coin_decimals: u64::from_le_bytes(*coin_decimals),
            pc_decimals: u64::from_le_bytes(*pc_decimals),
            state: u64::from_le_bytes(*state),
            reset_flag: u64::from_le_bytes(*reset_flag),
            min_size: u64::from_le_bytes(*min_size),
            vol_max_cut_ratio: u64::from_le_bytes(*vol_max_cut_ratio),
            amount_wave: u64::from_le_bytes(*amount_wave),
            coin_lot_size: u64::from_le_bytes(*coin_lot_size),
            pc_lot_size: u64::from_le_bytes(*pc_lot_size),
            min_price_multiplier: u64::from_le_bytes(*min_price_multiplier),
            max_price_multiplier: u64::from_le_bytes(*max_price_multiplier),
            sys_decimal_value: u64::from_le_bytes(*sys_decimal_value),
            min_separate_numerator: u64::from_le_bytes(*min_separate_numerator),
            min_separate_denominator: u64::from_le_bytes(*min_separate_denominator),
            trade_fee_numerator: u64::from_le_bytes(*trade_fee_numerator),
            trade_fee_denominator: u64::from_le_bytes(*trade_fee_denominator),
            pnl_numerator: u64::from_le_bytes(*pnl_numerator),
            pnl_denominator: u64::from_le_bytes(*pnl_denominator),
            swap_fee_numerator: u64::from_le_bytes(*swap_fee_numerator),
            swap_fee_denominator: u64::from_le_bytes(*swap_fee_denominator),
            need_take_pnl_coin: u64::from_le_bytes(*need_take_pnl_coin),
            need_take_pnl_pc: u64::from_le_bytes(*need_take_pnl_pc),
            total_pnl_pc: u64::from_le_bytes(*total_pnl_pc),
            total_pnl_coin: u64::from_le_bytes(*total_pnl_coin),
            pool_total_deposit_pc: u128::from_le_bytes(*pool_total_deposit_pc),
            pool_total_deposit_coin: u128::from_le_bytes(*pool_total_deposit_coin),
            swap_coin_in_amount: u128::from_le_bytes(*swap_coin_in_amount),
            swap_pc_out_amount: u128::from_le_bytes(*swap_pc_out_amount),
            swap_coin_to_pc_fee: u64::from_le_bytes(*swap_coin_to_pc_fee),
            swap_pc_in_amount: u128::from_le_bytes(*swap_pc_in_amount),
            swap_coin_out_amount: u128::from_le_bytes(*swap_coin_out_amount),
            swap_pc_to_coin_fee: u64::from_le_bytes(*swap_pc_to_coin_fee),
            token_coin: Pubkey::new_from_array(*token_coin),
            token_pc: Pubkey::new_from_array(*token_pc),
            coin_mint: Pubkey::new_from_array(*coin_mint),
            pc_mint: Pubkey::new_from_array(*pc_mint),
            lp_mint: Pubkey::new_from_array(*lp_mint),
            open_orders: Pubkey::new_from_array(*open_orders),
            market: Pubkey::new_from_array(*market),
            serum_dex: Pubkey::new_from_array(*serum_dex),
            target_orders: Pubkey::new_from_array(*target_orders),
            withdraw_queue: Pubkey::new_from_array(*withdraw_queue),
            token_temp_lp: Pubkey::new_from_array(*token_temp_lp),
            amm_owner: Pubkey::new_from_array(*amm_owner),
            pnl_owner: Pubkey::new_from_array(*pnl_owner),
        })
    }
}

pub fn check_pool_program_id(program_id: &Pubkey) -> bool {
    program_id == &raydium_v2::id()
        || program_id == &raydium_v3::id()
        || program_id == &raydium_v4::id()
}

pub fn check_stake_program_id(program_id: &Pubkey) -> bool {
    program_id == &raydium_stake::id()
        || program_id == &raydium_stake_v4::id()
        || program_id == &raydium_stake_v5::id()
}

/// Returns amount of LP tokens staked as recorded in the specified stake account
pub fn get_stake_account_balance(stake_account: &AccountInfo) -> Result<u64, ProgramError> {
    let data = stake_account.try_borrow_data()?;
    if data.len() == RaydiumUserStakeInfoV4::LEN {
        Ok(RaydiumUserStakeInfoV4::unpack(&data)?.deposit_balance)
    } else if data.len() == RaydiumUserStakeInfo::LEN {
        Ok(RaydiumUserStakeInfo::unpack(&data)?.deposit_balance)
    } else {
        Err(ProgramError::InvalidAccountData)
    }
}

pub fn get_pool_token_balances<'a, 'b>(
    pool_coin_token_account: &'a AccountInfo<'b>,
    pool_pc_token_account: &'a AccountInfo<'b>,
    amm_open_orders: &'a AccountInfo<'b>,
    amm_id: &'a AccountInfo<'b>,
) -> Result<(u64, u64), ProgramError> {
    // get token balances
    let mut token_a_balance = account::get_token_balance(pool_coin_token_account)?;
    let mut token_b_balance = account::get_token_balance(pool_pc_token_account)?;

    // adjust with open orders
    if amm_open_orders.data_len() == 3228 {
        let open_orders_data = amm_open_orders.try_borrow_data()?;
        let base_token_total = array_ref![open_orders_data, 85, 8];
        let quote_token_total = array_ref![open_orders_data, 101, 8];

        token_a_balance += u64::from_le_bytes(*base_token_total);
        token_b_balance += u64::from_le_bytes(*quote_token_total);
    }

    // adjust with amm take pnl
    let (pnl_coin_offset, pnl_pc_offset) = if amm_id.data_len() == 624 {
        (136, 144)
    } else if amm_id.data_len() == 680 {
        (144, 152)
    } else if amm_id.data_len() == 752 {
        (192, 200)
    } else {
        (0, 0)
    };
    if pnl_coin_offset > 0 {
        let amm_id_data = amm_id.try_borrow_data()?;
        let need_take_pnl_coin = u64::from_le_bytes(*array_ref![amm_id_data, pnl_coin_offset, 8]);
        let need_take_pnl_pc = u64::from_le_bytes(*array_ref![amm_id_data, pnl_pc_offset, 8]);

        // safe to use unchecked sub
        token_a_balance -= if need_take_pnl_coin < token_a_balance {
            need_take_pnl_coin
        } else {
            token_a_balance
        };
        // safe to use unchecked sub
        token_b_balance -= if need_take_pnl_pc < token_b_balance {
            need_take_pnl_pc
        } else {
            token_b_balance
        };
    }

    Ok((token_a_balance, token_b_balance))
}

pub fn get_pool_deposit_amounts<'a, 'b>(
    pool_coin_token_account: &'a AccountInfo<'b>,
    pool_pc_token_account: &'a AccountInfo<'b>,
    amm_open_orders: &'a AccountInfo<'b>,
    amm_id: &'a AccountInfo<'b>,
    max_coin_token_amount: u64,
    max_pc_token_amount: u64,
) -> Result<(u64, u64), ProgramError> {
    if max_coin_token_amount > 0 && max_pc_token_amount > 0 {
        return Ok((max_coin_token_amount, max_pc_token_amount));
    }
    if max_coin_token_amount == 0 && max_pc_token_amount == 0 {
        msg!("Error: At least one of token amounts must be non-zero");
        return Err(ProgramError::InvalidArgument);
    }
    let mut coin_token_amount = max_coin_token_amount;
    let mut pc_token_amount = max_pc_token_amount;
    let (coin_balance, pc_balance) = get_pool_token_balances(
        pool_coin_token_account,
        pool_pc_token_account,
        amm_open_orders,
        amm_id,
    )?;
    if coin_balance == 0 || pc_balance == 0 {
        msg!("Error: Both amounts must be specified for the initial deposit to an empty pool");
        return Err(ProgramError::InvalidArgument);
    }
    if max_coin_token_amount == 0 {
        let estimated_coin_amount = math::checked_as_u64(
            coin_balance as f64 * max_pc_token_amount as f64 / (pc_balance as f64),
        )?;
        coin_token_amount = if estimated_coin_amount > 1 {
            estimated_coin_amount - 1
        } else {
            0
        };
    } else {
        pc_token_amount = math::checked_as_u64(
            pc_balance as f64 * max_coin_token_amount as f64 / (coin_balance as f64),
        )?;
    }
    Ok((coin_token_amount, math::checked_add(pc_token_amount, 1)?))
}

pub fn get_pool_withdrawal_amounts<'a, 'b>(
    pool_coin_token_account: &'a AccountInfo<'b>,
    pool_pc_token_account: &'a AccountInfo<'b>,
    amm_open_orders: &'a AccountInfo<'b>,
    amm_id: &'a AccountInfo<'b>,
    lp_token_mint: &'a AccountInfo<'b>,
    lp_token_amount: u64,
) -> Result<(u64, u64), ProgramError> {
    if lp_token_amount == 0 {
        msg!("Error: LP token amount must be non-zero");
        return Err(ProgramError::InvalidArgument);
    }
    let (coin_balance, pc_balance) = get_pool_token_balances(
        pool_coin_token_account,
        pool_pc_token_account,
        amm_open_orders,
        amm_id,
    )?;
    if coin_balance == 0 && pc_balance == 0 {
        return Ok((0, 0));
    }
    let lp_token_supply = account::get_token_supply(lp_token_mint)?;
    if lp_token_supply == 0 {
        return Ok((0, 0));
    }
    let stake = lp_token_amount as f64 / lp_token_supply as f64;

    Ok((
        math::checked_as_u64(coin_balance as f64 * stake)?,
        math::checked_as_u64(pc_balance as f64 * stake)?,
    ))
}

pub fn get_pool_swap_amounts<'a, 'b>(
    pool_coin_token_account: &'a AccountInfo<'b>,
    pool_pc_token_account: &'a AccountInfo<'b>,
    amm_open_orders: &'a AccountInfo<'b>,
    amm_id: &'a AccountInfo<'b>,
    coin_token_amount_in: u64,
    pc_token_amount_in: u64,
) -> Result<(u64, u64), ProgramError> {
    if (coin_token_amount_in == 0 && pc_token_amount_in == 0)
        || (coin_token_amount_in > 0 && pc_token_amount_in > 0)
    {
        msg!("Error: One and only one of token amounts must be non-zero");
        return Err(ProgramError::InvalidArgument);
    }
    let (coin_balance, pc_balance) = get_pool_token_balances(
        pool_coin_token_account,
        pool_pc_token_account,
        amm_open_orders,
        amm_id,
    )?;
    if coin_balance == 0 || pc_balance == 0 {
        msg!("Error: Can't swap in an empty pool");
        return Err(ProgramError::Custom(412));
    }
    if coin_token_amount_in == 0 {
        // pc to coin
        let amount_in_no_fee = (pc_token_amount_in as f64 * (1.0 - RAYDIUM_FEE)) as u64;
        let estimated_coin_amount = math::checked_as_u64(
            coin_balance as f64 * amount_in_no_fee as f64
                / (pc_balance as f64 + amount_in_no_fee as f64),
        )?;
        Ok((
            pc_token_amount_in,
            if estimated_coin_amount > 1 {
                estimated_coin_amount - 1
            } else {
                0
            },
        ))
    } else {
        // coin to pc
        let amount_in_no_fee = (coin_token_amount_in as f64 * (1.0 - RAYDIUM_FEE)) as u64;
        let estimated_pc_amount = math::checked_as_u64(
            pc_balance as f64 * amount_in_no_fee as f64
                / (coin_balance as f64 + amount_in_no_fee as f64),
        )?;
        Ok((
            coin_token_amount_in,
            if estimated_pc_amount > 1 {
                estimated_pc_amount - 1
            } else {
                0
            },
        ))
    }
}

pub fn estimate_lp_tokens_amount(
    lp_token_mint: &AccountInfo,
    token_a_deposit: u64,
    token_b_deposit: u64,
    pool_coin_balance: u64,
    pool_pc_balance: u64,
) -> Result<u64, ProgramError> {
    if pool_coin_balance != 0 && pool_pc_balance != 0 {
        Ok(std::cmp::min(
            math::checked_as_u64(
                (token_a_deposit as f64 / pool_coin_balance as f64)
                    * account::get_token_supply(lp_token_mint)? as f64,
            )?,
            math::checked_as_u64(
                (token_b_deposit as f64 / pool_pc_balance as f64)
                    * account::get_token_supply(lp_token_mint)? as f64,
            )?,
        ))
    } else if pool_coin_balance != 0 {
        math::checked_as_u64(
            (token_a_deposit as f64 / pool_coin_balance as f64)
                * account::get_token_supply(lp_token_mint)? as f64,
        )
    } else if pool_pc_balance != 0 {
        math::checked_as_u64(
            (token_b_deposit as f64 / pool_pc_balance as f64)
                * account::get_token_supply(lp_token_mint)? as f64,
        )
    } else {
        Ok(0)
    }
}

pub fn add_liquidity(
    accounts: &[AccountInfo],
    max_coin_token_amount: u64,
    max_pc_token_amount: u64,
) -> ProgramResult {
    if let [user_account, user_token_a_account, user_token_b_account, user_lp_token_account, pool_program_id, pool_coin_token_account, pool_pc_token_account, lp_token_mint, spl_token_id, amm_id, amm_authority, amm_open_orders, amm_target, serum_market] =
        accounts
    {
        if !check_pool_program_id(pool_program_id.key) {
            return Err(ProgramError::IncorrectProgramId);
        }
        let raydium_accounts = vec![
            AccountMeta::new_readonly(*spl_token_id.key, false),
            AccountMeta::new(*amm_id.key, false),
            AccountMeta::new_readonly(*amm_authority.key, false),
            AccountMeta::new_readonly(*amm_open_orders.key, false),
            AccountMeta::new(*amm_target.key, false),
            AccountMeta::new(*lp_token_mint.key, false),
            AccountMeta::new(*pool_coin_token_account.key, false),
            AccountMeta::new(*pool_pc_token_account.key, false),
            AccountMeta::new_readonly(*serum_market.key, false),
            AccountMeta::new(*user_token_a_account.key, false),
            AccountMeta::new(*user_token_b_account.key, false),
            AccountMeta::new(*user_lp_token_account.key, false),
            AccountMeta::new_readonly(*user_account.key, true),
        ];

        let instruction = Instruction {
            program_id: *pool_program_id.key,
            accounts: raydium_accounts,
            data: RaydiumAddLiquidity {
                instruction: 3,
                max_coin_token_amount,
                max_pc_token_amount,
                base_side: 0,
            }
            .to_vec()?,
        };
        invoke(&instruction, accounts)
    } else {
        Err(ProgramError::NotEnoughAccountKeys)
    }
}

pub fn add_liquidity_with_seeds(
    accounts: &[AccountInfo],
    seeds: &[&[&[u8]]],
    max_coin_token_amount: u64,
    max_pc_token_amount: u64,
) -> ProgramResult {
    if let [authority_account, token_a_custody_account, token_b_custody_account, lp_token_custody_account, pool_program_id, pool_coin_token_account, pool_pc_token_account, lp_token_mint, spl_token_id, amm_id, amm_authority, amm_open_orders, amm_target, serum_market] =
        accounts
    {
        if !check_pool_program_id(pool_program_id.key) {
            return Err(ProgramError::IncorrectProgramId);
        }
        let raydium_accounts = vec![
            AccountMeta::new_readonly(*spl_token_id.key, false),
            AccountMeta::new(*amm_id.key, false),
            AccountMeta::new_readonly(*amm_authority.key, false),
            AccountMeta::new_readonly(*amm_open_orders.key, false),
            AccountMeta::new(*amm_target.key, false),
            AccountMeta::new(*lp_token_mint.key, false),
            AccountMeta::new(*pool_coin_token_account.key, false),
            AccountMeta::new(*pool_pc_token_account.key, false),
            AccountMeta::new_readonly(*serum_market.key, false),
            AccountMeta::new(*token_a_custody_account.key, false),
            AccountMeta::new(*token_b_custody_account.key, false),
            AccountMeta::new(*lp_token_custody_account.key, false),
            AccountMeta::new_readonly(*authority_account.key, true),
        ];

        let instruction = Instruction {
            program_id: *pool_program_id.key,
            accounts: raydium_accounts,
            data: RaydiumAddLiquidity {
                instruction: 3,
                max_coin_token_amount,
                max_pc_token_amount,
                base_side: 0,
            }
            .to_vec()?,
        };

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
    if let [authority_account, token_a_custody_account, token_b_custody_account, lp_token_custody_account, pool_program_id, pool_withdraw_queue, pool_temp_lp_token_account, pool_coin_token_account, pool_pc_token_account, lp_token_mint, spl_token_id, amm_id, amm_authority, amm_open_orders, amm_target, serum_market, serum_program_id, serum_coin_vault_account, serum_pc_vault_account, serum_vault_signer] =
        accounts
    {
        if !check_pool_program_id(pool_program_id.key) {
            return Err(ProgramError::IncorrectProgramId);
        }
        let raydium_accounts = vec![
            AccountMeta::new_readonly(*spl_token_id.key, false),
            AccountMeta::new(*amm_id.key, false),
            AccountMeta::new_readonly(*amm_authority.key, false),
            AccountMeta::new(*amm_open_orders.key, false),
            AccountMeta::new(*amm_target.key, false),
            AccountMeta::new(*lp_token_mint.key, false),
            AccountMeta::new(*pool_coin_token_account.key, false),
            AccountMeta::new(*pool_pc_token_account.key, false),
            AccountMeta::new(*pool_withdraw_queue.key, false),
            AccountMeta::new(*pool_temp_lp_token_account.key, false),
            AccountMeta::new_readonly(*serum_program_id.key, false),
            AccountMeta::new(*serum_market.key, false),
            AccountMeta::new(*serum_coin_vault_account.key, false),
            AccountMeta::new(*serum_pc_vault_account.key, false),
            AccountMeta::new_readonly(*serum_vault_signer.key, false),
            AccountMeta::new(*lp_token_custody_account.key, false),
            AccountMeta::new(*token_a_custody_account.key, false),
            AccountMeta::new(*token_b_custody_account.key, false),
            AccountMeta::new_readonly(*authority_account.key, true),
        ];

        let instruction = Instruction {
            program_id: *pool_program_id.key,
            accounts: raydium_accounts,
            data: RaydiumRemoveLiquidity {
                instruction: 4,
                amount: if amount > 0 {
                    amount
                } else {
                    account::get_token_balance(lp_token_custody_account)?
                },
            }
            .to_vec()?,
        };

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
    if let [authority_account, stake_info_account, lp_token_custody_account, token_a_custody_account, token_b_custody_account, pool_program_id, farm_lp_token_account, farm_reward_token_a_account, farm_reward_token_b_account, clock_id, spl_token_id, farm_id, farm_authority] =
        accounts
    {
        if !check_stake_program_id(pool_program_id.key) {
            return Err(ProgramError::IncorrectProgramId);
        }
        let mut raydium_accounts = vec![
            AccountMeta::new(*farm_id.key, false),
            AccountMeta::new_readonly(*farm_authority.key, false),
            AccountMeta::new(*stake_info_account.key, false),
            AccountMeta::new_readonly(*authority_account.key, true),
            AccountMeta::new(*lp_token_custody_account.key, false),
            AccountMeta::new(*farm_lp_token_account.key, false),
            AccountMeta::new(*token_a_custody_account.key, false),
            AccountMeta::new(*farm_reward_token_a_account.key, false),
            AccountMeta::new_readonly(*clock_id.key, false),
            AccountMeta::new_readonly(*spl_token_id.key, false),
        ];
        if *farm_reward_token_b_account.key != zero::id() {
            raydium_accounts.push(AccountMeta::new(*token_b_custody_account.key, false));
            raydium_accounts.push(AccountMeta::new(*farm_reward_token_b_account.key, false));
        }

        let instruction = Instruction {
            program_id: *pool_program_id.key,
            accounts: raydium_accounts,
            data: RaydiumStake {
                instruction: 1,
                amount,
            }
            .to_vec()?,
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
    if let [authority_account, token_a_custody_account, token_b_custody_account, pool_program_id, pool_coin_token_account, pool_pc_token_account, spl_token_id, amm_id, amm_authority, amm_open_orders, amm_target, serum_market, serum_program_id, serum_bids, serum_asks, serum_event_queue, serum_coin_vault_account, serum_pc_vault_account, serum_vault_signer] =
        accounts
    {
        if !check_pool_program_id(pool_program_id.key) {
            return Err(ProgramError::IncorrectProgramId);
        }
        let raydium_accounts = vec![
            AccountMeta::new_readonly(*spl_token_id.key, false),
            AccountMeta::new(*amm_id.key, false),
            AccountMeta::new_readonly(*amm_authority.key, false),
            AccountMeta::new(*amm_open_orders.key, false),
            AccountMeta::new(*amm_target.key, false),
            AccountMeta::new(*pool_coin_token_account.key, false),
            AccountMeta::new(*pool_pc_token_account.key, false),
            AccountMeta::new_readonly(*serum_program_id.key, false),
            AccountMeta::new(*serum_market.key, false),
            AccountMeta::new(*serum_bids.key, false),
            AccountMeta::new(*serum_asks.key, false),
            AccountMeta::new(*serum_event_queue.key, false),
            AccountMeta::new(*serum_coin_vault_account.key, false),
            AccountMeta::new(*serum_pc_vault_account.key, false),
            AccountMeta::new_readonly(*serum_vault_signer.key, false),
            AccountMeta::new(*token_a_custody_account.key, false),
            AccountMeta::new(*token_b_custody_account.key, false),
            AccountMeta::new_readonly(*authority_account.key, true),
        ];

        let instruction = Instruction {
            program_id: *pool_program_id.key,
            accounts: raydium_accounts,
            data: RaydiumSwap {
                instruction: 9,
                amount_in,
                min_amount_out,
            }
            .to_vec()?,
        };

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
    if let [authority_account, stake_info_account, lp_token_custody_account, token_a_custody_account, token_b_custody_account, pool_program_id, farm_lp_token_account, farm_reward_token_a_account, farm_reward_token_b_account, clock_id, spl_token_id, farm_id, farm_authority] =
        accounts
    {
        if !check_stake_program_id(pool_program_id.key) {
            return Err(ProgramError::IncorrectProgramId);
        }
        let mut raydium_accounts = vec![
            AccountMeta::new(*farm_id.key, false),
            AccountMeta::new_readonly(*farm_authority.key, false),
            AccountMeta::new(*stake_info_account.key, false),
            AccountMeta::new_readonly(*authority_account.key, true),
            AccountMeta::new(*lp_token_custody_account.key, false),
            AccountMeta::new(*farm_lp_token_account.key, false),
            AccountMeta::new(*token_a_custody_account.key, false),
            AccountMeta::new(*farm_reward_token_a_account.key, false),
            AccountMeta::new_readonly(*clock_id.key, false),
            AccountMeta::new_readonly(*spl_token_id.key, false),
        ];
        if *farm_reward_token_b_account.key != zero::id() {
            raydium_accounts.push(AccountMeta::new(*token_b_custody_account.key, false));
            raydium_accounts.push(AccountMeta::new(*farm_reward_token_b_account.key, false));
        }

        let instruction = Instruction {
            program_id: *pool_program_id.key,
            accounts: raydium_accounts,
            data: RaydiumUnstake {
                instruction: 2,
                amount,
            }
            .to_vec()?,
        };

        invoke_signed(&instruction, accounts, seeds)
    } else {
        Err(ProgramError::NotEnoughAccountKeys)
    }
}
