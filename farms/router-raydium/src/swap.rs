//! Swap tokens with the Raydium pool instruction

use {
    solana_farm_sdk::{
        instruction::raydium::RaydiumSwap,
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

pub fn swap(
    accounts: &[AccountInfo],
    token_a_amount_in: u64,
    token_b_amount_in: u64,
    min_token_amount_out: u64,
) -> ProgramResult {
    msg!("Processing AmmInstruction::Swap");
    msg!("token_a_amount_in {} ", token_a_amount_in);
    msg!("token_b_amount_in {} ", token_b_amount_in);
    msg!("min_token_amount_out {} ", min_token_amount_out);

    #[allow(clippy::deprecated_cfg_attr)]
    #[cfg_attr(rustfmt, rustfmt_skip)]
    if let [
        user_account,
        user_token_a_account,
        user_token_b_account,
        pool_program_id,
        pool_coin_token_account,
        pool_pc_token_account,
        spl_token_id,
        amm_id,
        amm_authority,
        amm_open_orders,
        amm_target,
        serum_market,
        serum_program_id,
        serum_bids,
        serum_asks,
        serum_event_queue,
        serum_coin_vault_account,
        serum_pc_vault_account,
        serum_vault_signer
        ] = accounts
    {
        if !raydium::check_pool_program_id(pool_program_id.key) {
            return Err(ProgramError::IncorrectProgramId);
        }

        let (amount_in, mut min_amount_out) = raydium::get_pool_swap_amounts(
            pool_coin_token_account,
            pool_pc_token_account,
            amm_open_orders,
            amm_id,
            token_a_amount_in,
            token_b_amount_in,
        )?;
        if min_token_amount_out > min_amount_out {
            min_amount_out = min_token_amount_out;
        }

        let initial_balance_in = if token_a_amount_in == 0 {
            account::get_token_balance(user_token_b_account)?
        } else {
            account::get_token_balance(user_token_a_account)?
        };
        let initial_balance_out = if token_a_amount_in == 0 {
            account::get_token_balance(user_token_a_account)?
        } else {
            account::get_token_balance(user_token_b_account)?
        };

        let mut raydium_accounts = Vec::with_capacity(18);
        raydium_accounts.push(AccountMeta::new_readonly(*spl_token_id.key, false));
        raydium_accounts.push(AccountMeta::new(*amm_id.key, false));
        raydium_accounts.push(AccountMeta::new_readonly(*amm_authority.key, false));
        raydium_accounts.push(AccountMeta::new(*amm_open_orders.key, false));
        raydium_accounts.push(AccountMeta::new(*amm_target.key, false));
        raydium_accounts.push(AccountMeta::new(*pool_coin_token_account.key, false));
        raydium_accounts.push(AccountMeta::new(*pool_pc_token_account.key, false));
        raydium_accounts.push(AccountMeta::new_readonly(*serum_program_id.key, false));
        raydium_accounts.push(AccountMeta::new(*serum_market.key, false));
        raydium_accounts.push(AccountMeta::new(*serum_bids.key, false));
        raydium_accounts.push(AccountMeta::new(*serum_asks.key, false));
        raydium_accounts.push(AccountMeta::new(*serum_event_queue.key, false));
        raydium_accounts.push(AccountMeta::new(*serum_coin_vault_account.key, false));
        raydium_accounts.push(AccountMeta::new(*serum_pc_vault_account.key, false));
        raydium_accounts.push(AccountMeta::new_readonly(*serum_vault_signer.key, false));
        if token_a_amount_in == 0 {
            raydium_accounts.push(AccountMeta::new(*user_token_b_account.key, false));
            raydium_accounts.push(AccountMeta::new(*user_token_a_account.key, false));
        } else {
            raydium_accounts.push(AccountMeta::new(*user_token_a_account.key, false));
            raydium_accounts.push(AccountMeta::new(*user_token_b_account.key, false));
        }
        raydium_accounts.push(AccountMeta::new_readonly(*user_account.key, true));

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
        invoke(&instruction, accounts)?;

        account::check_tokens_spent(
            if token_a_amount_in == 0 {
                user_token_b_account
            } else {
                user_token_a_account
            },
            initial_balance_in,
            amount_in,
        )?;
        account::check_tokens_received(
            if token_a_amount_in == 0 {
                user_token_a_account
            } else {
                user_token_b_account
            },
            initial_balance_out,
            min_amount_out,
        )?;
    } else {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    msg!("AmmInstruction::Swap complete");
    Ok(())
}
