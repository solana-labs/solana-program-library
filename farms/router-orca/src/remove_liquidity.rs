//! Remove liquidity from the Orca pool instruction

use {
    solana_farm_sdk::program::{account, protocol::orca},
    solana_program::{
        account_info::AccountInfo, entrypoint::ProgramResult, msg, program::invoke,
        program_error::ProgramError,
    },
    spl_token_swap::instruction,
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
        amm_id,
        amm_authority,
        fees_account
        ] = accounts
    {
        if !orca::check_pool_program_id(pool_program_id.key) {
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

        let (token_a_amount, token_b_amount) = orca::get_pool_withdrawal_amounts(
            pool_token_a_account,
            pool_token_b_account,
            lp_token_mint,
            lp_amount,
        )?;

        let data = instruction::WithdrawAllTokenTypes {
            pool_token_amount: lp_amount,
            minimum_token_a_amount: token_a_amount,
            minimum_token_b_amount: token_b_amount,
        };

        msg!(
            "Removing tokens from the pool. lp_amount: {}, token_a_amount: {}, token_b_amount: {}",
            lp_amount,
            token_a_amount,
            token_b_amount
        );
        let instruction = instruction::withdraw_all_token_types(
            pool_program_id.key,
            &spl_token::id(),
            amm_id.key,
            amm_authority.key,
            user_account.key,
            lp_token_mint.key,
            fees_account.key,
            user_lp_token_account.key,
            pool_token_a_account.key,
            pool_token_b_account.key,
            user_token_a_account.key,
            user_token_b_account.key,
            data,
        )?;

        invoke(&instruction, accounts)?;

        account::check_tokens_spent(
            user_lp_token_account,
            initial_lp_token_user_balance,
            lp_amount,
        )?;
        account::check_tokens_received(
            user_token_a_account,
            initial_token_a_user_balance,
            token_a_amount,
        )?;
        account::check_tokens_received(
            user_token_b_account,
            initial_token_b_user_balance,
            token_b_amount,
        )?;
    } else {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    msg!("AmmInstruction::RemoveLiquidity complete");
    Ok(())
}
