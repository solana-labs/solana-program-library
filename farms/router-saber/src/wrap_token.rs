//! Wrap token to a Saber decimal token instruction

use {
    solana_farm_sdk::program::{account, protocol::saber},
    solana_program::{
        account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
    },
};

pub fn wrap_token(accounts: &[AccountInfo], amount: u64) -> ProgramResult {
    msg!("Processing AmmInstruction::WrapToken");
    msg!("amount {} ", amount);

    #[allow(clippy::deprecated_cfg_attr)]
    #[cfg_attr(rustfmt, rustfmt_skip)]
    if let [
        user_account,
        user_underlying_token_account,
        underlying_token_mint,
        _spl_token_id,
        decimal_wrapper_program,
        user_wrapped_token_account,
        wrapped_token_mint,
        wrapped_token_vault,
        decimal_wrapper
        ] = accounts
    {
        let initial_underlying_token_user_balance =
            account::get_token_balance(user_underlying_token_account)?;
        let initial_wrapped_token_user_balance =
            account::get_token_balance(user_wrapped_token_account)?;

        let underlying_decimals = account::get_token_decimals(underlying_token_mint)?;
        let wrapped_decimals = account::get_token_decimals(wrapped_token_mint)?;

        saber::wrap_token(
            decimal_wrapper,
            wrapped_token_mint,
            wrapped_token_vault,
            user_account,
            user_underlying_token_account,
            user_wrapped_token_account,
            decimal_wrapper_program.key,
            amount,
        )?;

        account::check_tokens_spent(
            user_underlying_token_account,
            initial_underlying_token_user_balance,
            amount,
        )?;
        account::check_tokens_received(
            user_wrapped_token_account,
            initial_wrapped_token_user_balance,
            account::to_amount_with_new_decimals(amount, underlying_decimals, wrapped_decimals)?,
        )?;
    } else {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    msg!("AmmInstruction::WrapToken complete");
    Ok(())
}
