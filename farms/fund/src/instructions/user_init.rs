//! Initialize a new user for the Fund instruction handler
use {
    crate::user_info::UserInfo,
    solana_farm_sdk::{
        fund::{Fund, FundUserAction, FundUserRequests, DISCRIMINATOR_FUND_USER_REQUESTS},
        id::main_router,
        program::{account, pda},
        string::ArrayString64,
        token::Token,
        traits::Packed,
    },
    solana_program::{
        account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
    },
};

pub fn user_init(fund: &Fund, accounts: &[AccountInfo]) -> ProgramResult {
    #[allow(clippy::deprecated_cfg_attr)]
    #[cfg_attr(rustfmt, rustfmt_skip)]
    if let [
        funding_account,
        fund_metadata,
        _fund_info_account,
        user_account,
        user_info_account,
        user_requests_account,
        custody_token_ref,
        _system_program
        ] = accounts
    {
        // validate params and accounts
        if account::exists(user_requests_account)? {
            msg!("Error: User already initialized");
            return Err(ProgramError::AccountAlreadyInitialized);
        }
        if custody_token_ref.owner != &main_router::id() {
            msg!("Error: Invalid custody token metadata account");
            return Err(ProgramError::IllegalOwner);
        }

        // create user info account
        if account::is_empty(user_info_account)? {
            msg!("Create user info account");
            let seeds: &[&[u8]] = &[
                b"user_info_account",
                user_account.key.as_ref(),
                fund.name.as_bytes(),
            ];
            let bump = pda::init_system_account(
                funding_account,
                user_info_account,
                &fund.fund_program_id,
                &fund.fund_program_id,
                seeds,
                UserInfo::LEN,
            )?;
            let mut user_info = UserInfo::new(user_info_account);
            user_info.init(&fund.name, bump)?;
        } else if !UserInfo::validate_account(fund, user_info_account, user_account.key) {
            msg!("Error: User info account already initialized but not valid");
            return Err(ProgramError::AccountAlreadyInitialized);
        }

        // create user requests account
        msg!("Create user requests account");
        let custody_token = account::unpack::<Token>(custody_token_ref, "custody token")?;
        let seeds: &[&[u8]] = &[
            b"user_requests_account",
            custody_token.name.as_bytes(),
            user_account.key.as_ref(),
            fund.name.as_bytes(),
        ];
        let bump = pda::init_system_account(
            funding_account,
            user_requests_account,
            &fund.fund_program_id,
            &fund.fund_program_id,
            seeds,
            FundUserRequests::LEN,
        )?;
        let user_requests = FundUserRequests {
            discriminator: DISCRIMINATOR_FUND_USER_REQUESTS,
            fund_ref: *fund_metadata.key,
            token_ref: *custody_token_ref.key,
            deposit_request: FundUserAction::default(),
            last_deposit: FundUserAction::default(),
            withdrawal_request: FundUserAction::default(),
            last_withdrawal: FundUserAction::default(),
            deny_reason: ArrayString64::default(),
            bump,
        };
        user_requests.pack(*user_requests_account.try_borrow_mut_data()?)?;
    } else {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    Ok(())
}
