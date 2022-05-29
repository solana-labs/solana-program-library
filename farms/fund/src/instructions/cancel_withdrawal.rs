//! Cancel withdrawal from the Fund instruction handler

use {
    crate::common,
    solana_farm_sdk::{
        fund::{Fund, FundUserRequests},
        id::main_router,
        program::account,
        string::ArrayString64,
        token::Token,
        traits::Packed,
    },
    solana_program::{
        account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
    },
};

pub fn cancel_withdrawal(fund: &Fund, accounts: &[AccountInfo]) -> ProgramResult {
    #[allow(clippy::deprecated_cfg_attr)]
    #[cfg_attr(rustfmt, rustfmt_skip)]
    if let [
        user_account,
        _fund_metadata,
        _fund_info_account,
        _spl_token_program,
        user_requests_account,
        user_fund_token_account,
        custody_token_metadata
        ] = accounts
    {
        // validate accounts
        msg!("Validate state and accounts");
        if !user_account.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }
        if custody_token_metadata.owner != &main_router::id() {
            msg!("Error: Invalid custody token metadata owner");
            return Err(ProgramError::IllegalOwner);
        }
        let custody_token = account::unpack::<Token>(custody_token_metadata, "custody token")?;
        let mut user_requests = account::unpack::<FundUserRequests>(user_requests_account, "user requests")?;
        common::check_user_requests_account(
            fund,
            &custody_token,
            &user_requests,
            user_account,
            user_requests_account,
        )?;

        // check if there are any pending requests
        if user_requests.withdrawal_request.amount == 0 {
            msg!("No pending withdrawals found");
            return Ok(());
        }

        // cancel pending withdrawal
        msg!("Cancel pending withdrawal");
        account::revoke_delegate(user_fund_token_account, user_account)?;
        user_requests.withdrawal_request.time = 0;
        user_requests.withdrawal_request.amount = 0;
        user_requests.deny_reason = ArrayString64::default();
        user_requests.pack(*user_requests_account.try_borrow_mut_data()?)?;

        Ok(())
    } else {
        Err(ProgramError::NotEnoughAccountKeys)
    }
}
