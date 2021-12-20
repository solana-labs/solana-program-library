//! Add liquidity to the Orca pool instruction

use {
    solana_farm_sdk::program::{account, protocol::orca},
    solana_program::{
        account_info::AccountInfo, entrypoint::ProgramResult, msg, program::invoke,
        program_error::ProgramError,
    },
    spl_token_swap::instruction,
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
        amm_id,
        amm_authority
        ] = accounts
    {
        if !orca::check_pool_program_id(pool_program_id.key) {
            return Err(ProgramError::IncorrectProgramId);
        }

        let (lp_token_amount, token_a_amount, token_b_amount) = orca::get_pool_deposit_amounts(
            pool_token_a_account,
            pool_token_b_account,
            lp_token_mint,
            max_token_a_amount,
            max_token_b_amount,
        )?;

        let initial_token_a_user_balance = account::get_token_balance(user_token_a_account)?;
        let initial_token_b_user_balance = account::get_token_balance(user_token_b_account)?;
        let initial_lp_token_user_balance = account::get_token_balance(user_lp_token_account)?;

        let data = instruction::DepositAllTokenTypes {
            pool_token_amount: lp_token_amount,
            maximum_token_a_amount: token_a_amount,
            maximum_token_b_amount: token_b_amount,
        };

        msg!("Deposit tokens into the pool. lp_token_amount: {}, token_a_amount: {}, token_b_amount: {}", lp_token_amount, token_a_amount, token_b_amount);
        let instruction = instruction::deposit_all_token_types(
            pool_program_id.key,
            &spl_token::id(),
            amm_id.key,
            amm_authority.key,
            user_account.key,
            user_token_a_account.key,
            user_token_b_account.key,
            pool_token_a_account.key,
            pool_token_b_account.key,
            lp_token_mint.key,
            user_lp_token_account.key,
            data,
        )?;

        invoke(&instruction, accounts)?;

        account::check_tokens_spent(
            user_token_a_account,
            initial_token_a_user_balance,
            token_a_amount,
        )?;
        account::check_tokens_spent(
            user_token_b_account,
            initial_token_b_user_balance,
            token_b_amount,
        )?;
        account::check_tokens_received(
            user_lp_token_account,
            initial_lp_token_user_balance,
            lp_token_amount,
        )?;
    } else {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    msg!("AmmInstruction::AddLiquidity complete");
    Ok(())
}
