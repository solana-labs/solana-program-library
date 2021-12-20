//! Remove liquidity from the Raydium pool instruction

use {
    solana_farm_sdk::{
        instruction::raydium::RaydiumRemoveLiquidity,
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

pub fn remove_liquidity(accounts: &[AccountInfo], amount: u64) -> ProgramResult {
    msg!("Processing AmmInstruction::RemoveLiquidity");
    msg!("amount {} ", amount);

    #[allow(clippy::deprecated_cfg_attr)]
    #[cfg_attr(rustfmt, rustfmt_skip)]
    if let [
        user_account,
        user_token_a_account,
        user_token_b_account,
        user_lp_token_account,
        pool_program_id,
        pool_withdraw_queue,
        pool_temp_lp_token_account,
        pool_coin_token_account,
        pool_pc_token_account,
        lp_token_mint,
        spl_token_id,
        amm_id,
        amm_authority,
        amm_open_orders,
        amm_target,
        serum_market,
        serum_program_id,
        serum_coin_vault_account,
        serum_pc_vault_account,
        serum_vault_signer
        ] = accounts
    {
        if !raydium::check_pool_program_id(pool_program_id.key) {
            return Err(ProgramError::IncorrectProgramId);
        }
        let initial_token_a_user_balance = account::get_token_balance(user_token_a_account)?;
        let initial_token_b_user_balance = account::get_token_balance(user_token_b_account)?;
        let initial_lp_token_user_balance = account::get_token_balance(user_lp_token_account)?;

        let lp_amount = if amount > 0 {
            amount
        } else {
            account::get_token_balance(user_lp_token_account)?
        };

        let (coin_token_amount, pc_token_amount) = raydium::get_pool_withdrawal_amounts(
            pool_coin_token_account,
            pool_pc_token_account,
            amm_open_orders,
            amm_id,
            lp_token_mint,
            lp_amount,
        )?;

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
            AccountMeta::new(*user_lp_token_account.key, false),
            AccountMeta::new(*user_token_a_account.key, false),
            AccountMeta::new(*user_token_b_account.key, false),
            AccountMeta::new_readonly(*user_account.key, true)
        ];

        let instruction = Instruction {
            program_id: *pool_program_id.key,
            accounts: raydium_accounts,
            data: RaydiumRemoveLiquidity {
                instruction: 4,
                amount: lp_amount,
            }
            .to_vec()?,
        };
        invoke(&instruction, accounts)?;

        account::check_tokens_spent(
            user_lp_token_account,
            initial_lp_token_user_balance,
            lp_amount,
        )?;
        account::check_tokens_received(
            user_token_a_account,
            initial_token_a_user_balance,
            coin_token_amount,
        )?;
        account::check_tokens_received(
            user_token_b_account,
            initial_token_b_user_balance,
            pc_token_amount,
        )?;
    } else {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    msg!("AmmInstruction::RemoveLiquidity complete");
    Ok(())
}
