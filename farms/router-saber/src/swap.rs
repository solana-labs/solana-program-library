//! Swap tokens with the Saber pool instruction

use {
    solana_farm_sdk::program::account,
    solana_program::{
        account_info::AccountInfo, entrypoint::ProgramResult, msg, program::invoke,
        program_error::ProgramError,
    },
    stable_swap_client::instruction,
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
        pool_token_a_account,
        pool_token_b_account,
        _spl_token_id,
        _clock_id,
        swap_account,
        swap_authority,
        fees_account_a,
        fees_account_b
        ] = accounts
    {
        if &stable_swap_client::id() != pool_program_id.key {
            return Err(ProgramError::IncorrectProgramId);
        }

        let amount_in = if token_a_amount_in == 0 {
            token_b_amount_in
        } else {
            token_a_amount_in
        };

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

        let instruction = if token_a_amount_in > 0 {
            instruction::swap(
                &spl_token::id(),
                swap_account.key,
                swap_authority.key,
                user_account.key,
                user_token_a_account.key,
                pool_token_a_account.key,
                pool_token_b_account.key,
                user_token_b_account.key,
                fees_account_b.key,
                amount_in,
                min_token_amount_out,
            )?
        } else {
            instruction::swap(
                &spl_token::id(),
                swap_account.key,
                swap_authority.key,
                user_account.key,
                user_token_b_account.key,
                pool_token_b_account.key,
                pool_token_a_account.key,
                user_token_a_account.key,
                fees_account_a.key,
                amount_in,
                min_token_amount_out,
            )?
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
            min_token_amount_out,
        )?;
    } else {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    msg!("AmmInstruction::Swap complete");
    Ok(())
}
