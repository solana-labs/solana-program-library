//! Add liquidity to the Saber pool instruction

use {
    solana_farm_sdk::program::account,
    solana_program::{
        account_info::AccountInfo, entrypoint::ProgramResult, msg, program::invoke,
        program_error::ProgramError,
    },
    stable_swap_client::instruction,
};

pub fn add_liquidity(
    accounts: &[AccountInfo],
    max_token_a_amount: u64,
    max_token_b_amount: u64,
) -> ProgramResult {
    msg!("Processing AmmInstruction::AddLiquidity");
    msg!("max_token_a_amount {} ", max_token_a_amount);
    msg!("max_token_b_amount {} ", max_token_b_amount);

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
        _clock_id,
        swap_account,
        swap_authority
        ] = accounts
    {
        if &stable_swap_client::id() != pool_program_id.key {
            return Err(ProgramError::IncorrectProgramId);
        }

        let initial_token_a_user_balance = account::get_token_balance(user_token_a_account)?;
        let initial_token_b_user_balance = account::get_token_balance(user_token_b_account)?;
        let initial_lp_token_user_balance = account::get_token_balance(user_lp_token_account)?;

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

        invoke(&instruction, accounts)?;

        account::check_tokens_spent(
            user_token_a_account,
            initial_token_a_user_balance,
            max_token_a_amount,
        )?;
        account::check_tokens_spent(
            user_token_b_account,
            initial_token_b_user_balance,
            max_token_b_amount,
        )?;
        account::check_tokens_received(user_lp_token_account, initial_lp_token_user_balance, 1)?;
    } else {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    msg!("AmmInstruction::AddLiquidity complete");
    Ok(())
}
