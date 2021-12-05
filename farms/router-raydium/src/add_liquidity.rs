//! Add liquidity to the Raydium pool instruction

use {
    solana_farm_sdk::{
        instruction::raydium::RaydiumAddLiquidity,
        program::{account, protocol::raydium},
    },
    solana_program::{
        account_info::AccountInfo,
        entrypoint::ProgramResult,
        instruction::{AccountMeta, Instruction},
        msg,
        program::invoke,
        program_error::ProgramError,
    },
};

pub fn add_liquidity(
    accounts: &[AccountInfo],
    max_coin_token_amount: u64,
    max_pc_token_amount: u64,
) -> ProgramResult {
    msg!("Processing AmmInstruction::AddLiquidity");
    msg!("max_coin_token_amount {} ", max_coin_token_amount);
    msg!("max_pc_token_amount {} ", max_pc_token_amount);

    #[allow(clippy::deprecated_cfg_attr)]
    #[cfg_attr(rustfmt, rustfmt_skip)]
    if let [
        user_account,
        user_token_a_account,
        user_token_b_account,
        user_lp_token_account,
        pool_program_id,
        pool_coin_token_account,
        pool_pc_token_account,
        lp_token_mint,
        spl_token_id,
        amm_id,
        amm_authority,
        amm_open_orders,
        amm_target,
        serum_market
        ] = accounts
    {
        if !raydium::check_pool_program_id(pool_program_id.key) {
            return Err(ProgramError::IncorrectProgramId);
        }

        let (coin_token_amount, pc_token_amount) = raydium::get_pool_deposit_amounts(
            pool_coin_token_account,
            pool_pc_token_account,
            amm_open_orders,
            amm_id,
            max_coin_token_amount,
            max_pc_token_amount,
        )?;

        let initial_token_a_user_balance = account::get_token_balance(user_token_a_account)?;
        let initial_token_b_user_balance = account::get_token_balance(user_token_b_account)?;
        let initial_lp_token_user_balance = account::get_token_balance(user_lp_token_account)?;

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
            AccountMeta::new_readonly(*user_account.key, true)
        ];

        let instruction = Instruction {
            program_id: *pool_program_id.key,
            accounts: raydium_accounts,
            data: RaydiumAddLiquidity {
                instruction: 3,
                max_coin_token_amount: coin_token_amount,
                max_pc_token_amount: pc_token_amount,
                base_side: 0,
            }
            .to_vec()?,
        };
        invoke(&instruction, accounts)?;

        account::check_tokens_spent(
            user_token_a_account,
            initial_token_a_user_balance,
            coin_token_amount,
        )?;
        account::check_tokens_spent(
            user_token_b_account,
            initial_token_b_user_balance,
            pc_token_amount,
        )?;
        account::check_tokens_received(user_lp_token_account, initial_lp_token_user_balance, 1)?;
    } else {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    msg!("AmmInstruction::AddLiquidity complete");
    Ok(())
}
