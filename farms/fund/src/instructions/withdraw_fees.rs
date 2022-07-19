//! Fund WithdrawFees instruction handler

use {
    crate::fund_info::FundInfo,
    solana_farm_sdk::{
        fund::Fund,
        program::{account, pda},
    },
    solana_program::{
        account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
    },
};

pub fn withdraw_fees(fund: &Fund, accounts: &[AccountInfo], amount: u64) -> ProgramResult {
    #[allow(clippy::deprecated_cfg_attr)]
    #[cfg_attr(rustfmt, rustfmt_skip)]
    if let [
        _admin_account,
        _fund_metadata,
        fund_info_account,
        _active_multisig_account,
        fund_multisig_account,
        _spl_token_program,
        custody_fees_account,
        receiver
        ] = accounts
    {
        // validate accounts
        msg!("Validate state and accounts");
        if !account::check_token_account_owner(custody_fees_account, fund_multisig_account.key)? {
            msg!("Error: Invalid custody fees token account owner");
            return Err(ProgramError::IllegalOwner);
        }

        // transfer tokens
        msg!("Transfer fees from custody");
        let withdraw_amount = if amount > 0 {
            amount
        } else {
            account::get_token_balance(custody_fees_account)?
        };

        let seeds: &[&[&[u8]]] = &[&[
            b"multisig",
            fund.name.as_bytes(),
            &[fund.multisig_bump],
        ]];
        pda::transfer_tokens_with_seeds(
            custody_fees_account,
            receiver,
            fund_multisig_account,
            seeds,
            withdraw_amount,
        )?;

        // update fund stats
        msg!("Update Fund stats");
        let mut fund_info = FundInfo::new(fund_info_account);
        fund_info.update_admin_action_time()
    } else {
        Err(ProgramError::NotEnoughAccountKeys)
    }
}
