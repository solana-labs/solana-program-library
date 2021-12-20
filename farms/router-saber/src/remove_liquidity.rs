//! Remove liquidity from the Saber pool instruction

use {
    solana_farm_sdk::program::account,
    solana_program::{
        account_info::AccountInfo, entrypoint::ProgramResult, msg, program::invoke,
        program_error::ProgramError,
    },
    stable_swap_client::instruction,
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
        pool_token_a_account,
        pool_token_b_account,
        lp_token_mint,
        _spl_token_id,
        swap_account,
        swap_authority,
        fees_account_a,
        fees_account_b
        ] = accounts
    {
        if &stable_swap_client::id() != pool_program_id.key {
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

        let instruction = instruction::withdraw(
            &spl_token::id(),
            swap_account.key,
            swap_authority.key,
            user_account.key,
            lp_token_mint.key,
            user_lp_token_account.key,
            pool_token_a_account.key,
            pool_token_b_account.key,
            user_token_a_account.key,
            user_token_b_account.key,
            fees_account_a.key,
            fees_account_b.key,
            lp_amount,
            1,
            1,
        )?;

        invoke(&instruction, accounts)?;

        account::check_tokens_spent(
            user_lp_token_account,
            initial_lp_token_user_balance,
            lp_amount,
        )?;
        account::check_tokens_received(user_token_a_account, initial_token_a_user_balance, 1)?;
        account::check_tokens_received(user_token_b_account, initial_token_b_user_balance, 1)?;
    } else {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    msg!("AmmInstruction::RemoveLiquidity complete");
    Ok(())
}
