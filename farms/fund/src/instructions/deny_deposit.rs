//! Deny deposit to the Fund instruction handler

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

pub fn deny_deposit(
    fund: &Fund,
    accounts: &[AccountInfo],
    deny_reason: &ArrayString64,
) -> ProgramResult {
    #[allow(clippy::deprecated_cfg_attr)]
    #[cfg_attr(rustfmt, rustfmt_skip)]
    if let [
        _admin_account,
        _fund_metadata,
        _fund_info_account,
        _multisig_account,
        user_account,
        user_requests_account,
        custody_token_metadata
        ] = accounts
    {
        // validate params and accounts
        msg!("Validate state and accounts");
        let mut user_requests = account::unpack::<FundUserRequests>(user_requests_account, "user requests")?;
        if custody_token_metadata.owner != &main_router::id() {
            msg!("Error: Invalid custody token metadata owner");
            return Err(ProgramError::IllegalOwner);
        }
        let custody_token = account::unpack::<Token>(custody_token_metadata, "custody token")?;
        common::check_user_requests_account(
            fund,
            &custody_token,
            &user_requests,
            user_account,
            user_requests_account,
        )?;

        // check if there are any pending requests
        if user_requests.deposit_request.amount == 0 {
            msg!("Error: No pending deposits found");
            return Err(ProgramError::Custom(525));
        }

        // update user stats
        msg!("Update user stats");
        user_requests.last_deposit.time = user_requests.deposit_request.time;
        user_requests.last_deposit.amount = user_requests.deposit_request.amount;
        user_requests.deposit_request.time = 0;
        user_requests.deposit_request.amount = 0;
        user_requests.deny_reason = *deny_reason;
        user_requests.pack(*user_requests_account.try_borrow_mut_data()?)?;

        Ok(())
    } else {
        Err(ProgramError::NotEnoughAccountKeys)
    }
}
