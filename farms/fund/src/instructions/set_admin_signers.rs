//! Set admin signers instruction handler

use {
    crate::fund_info::FundInfo,
    solana_farm_sdk::{
        error::FarmError,
        fund::Fund,
        program::{account, multisig, multisig::Multisig, pda},
    },
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint::ProgramResult,
        msg,
    },
};

pub fn set_admin_signers(
    fund: &Fund,
    accounts: &[AccountInfo],
    min_signatures: u8,
) -> ProgramResult {
    msg!("Validate state and accounts");
    let accounts_iter = &mut accounts.iter();

    let signer_account = next_account_info(accounts_iter)?;
    let _fund_metadata = next_account_info(accounts_iter)?;
    let fund_info_account = next_account_info(accounts_iter)?;
    let _active_multisig_account = next_account_info(accounts_iter)?;
    let fund_multisig_account = next_account_info(accounts_iter)?;
    let _system_program = next_account_info(accounts_iter)?;

    if fund_multisig_account.key != &fund.multisig_account {
        msg!("Error: Invalid fund multisig account");
        return Err(FarmError::IncorrectAccountAddress.into());
    }

    if account::is_empty(fund_multisig_account)? {
        msg!("Init multisig account");
        let seeds: &[&[u8]] = &[b"multisig", fund.name.as_bytes()];
        let _bump = pda::init_system_account(
            signer_account,
            fund_multisig_account,
            &fund.fund_program_id,
            &fund.fund_program_id,
            seeds,
            Multisig::LEN,
        )?;
    } else {
        msg!("Update multisig account");
    }
    multisig::set_signers(
        fund_multisig_account,
        accounts_iter.as_slice(),
        min_signatures,
    )?;

    // update fund stats
    msg!("Update Fund stats");
    let mut fund_info = FundInfo::new(fund_info_account);
    fund_info.update_admin_action_time()
}
