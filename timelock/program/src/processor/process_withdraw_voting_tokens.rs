//! Program state processor

use crate::{error::TimelockError, state::timelock_program::TimelockProgram, state::{ timelock_set::TimelockSet}, utils::{ TokenTransferParams, assert_governance, assert_initialized, assert_same_version_as_program, spl_token_transfer}};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    pubkey::Pubkey,
};
use spl_token::state::{Account};

/// Withdraw voting tokens
pub fn process_withdraw_voting_tokens(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    voting_token_amount: u64,
) -> ProgramResult {

    let account_info_iter = &mut accounts.iter();
    let voting_account_info = next_account_info(account_info_iter)?;
    let yes_voting_account_info = next_account_info(account_info_iter)?;
    let no_voting_account_info = next_account_info(account_info_iter)?;
    let destination_governance_account_info = next_account_info(account_info_iter)?;
    let governance_holding_account_info = next_account_info(account_info_iter)?;
    let voting_dump_account_info = next_account_info(account_info_iter)?;
    let yes_voting_dump_account_info = next_account_info(account_info_iter)?;
    let no_voting_dump_account_info = next_account_info(account_info_iter)?;
    let timelock_set_account_info = next_account_info(account_info_iter)?;
    let transfer_authority_info = next_account_info(account_info_iter)?;
    let timelock_program_authority_info = next_account_info(account_info_iter)?;
    let timelock_program_account_info = next_account_info(account_info_iter)?;
    let token_program_account_info = next_account_info(account_info_iter)?;

    let timelock_set: TimelockSet = assert_initialized(timelock_set_account_info)?;
    let timelock_program: TimelockProgram = assert_initialized(timelock_program_account_info)?;

    if voting_token_amount < 0 as u64  {
        return Err(TimelockError::TokenAmountBelowZero.into());
    }

    assert_same_version_as_program(&timelock_program, &timelock_set)?;
    assert_governance(&timelock_set)?;

    let voting_account: Account = assert_initialized(voting_account_info)?;
    let yes_voting_account: Account = assert_initialized(yes_voting_account_info)?;
    let no_voting_account: Account = assert_initialized(no_voting_account_info)?;
    let _voting_dump: Account = assert_initialized(voting_dump_account_info)?;
    let _yes_voting_dump: Account = assert_initialized(yes_voting_dump_account_info)?;
    let _no_voting_dump: Account = assert_initialized(no_voting_dump_account_info)?;
    let _destination_governance_account: Account = assert_initialized(destination_governance_account_info)?;
    let _governance_account: Account = assert_initialized(governance_holding_account_info)?;

    let (authority_key, bump_seed) =
        Pubkey::find_program_address(&[timelock_program_account_info.key.as_ref()], program_id);
    if timelock_program_authority_info.key != &authority_key {
        return Err(TimelockError::InvalidTimelockAuthority.into());
    }
    let authority_signer_seeds = &[timelock_program_account_info.key.as_ref(), &[bump_seed]];

    let total_reimbursement = voting_account.amount + yes_voting_account.amount + no_voting_account.amount;

    spl_token_transfer(TokenTransferParams {
        source: voting_account_info.clone(),
        destination: voting_dump_account_info.clone(),
        amount: voting_account.amount,
        authority: transfer_authority_info.clone(),
        authority_signer_seeds: authority_signer_seeds,
        token_program: token_program_account_info.clone(),
    })?;


    spl_token_transfer(TokenTransferParams {
        source: yes_voting_account_info.clone(),
        destination: yes_voting_dump_account_info.clone(),
        amount: yes_voting_account.amount,
        authority: transfer_authority_info.clone(),
        authority_signer_seeds: authority_signer_seeds,
        token_program: token_program_account_info.clone(),
    })?;

    spl_token_transfer(TokenTransferParams {
        source: no_voting_account_info.clone(),
        destination: no_voting_dump_account_info.clone(),
        amount: no_voting_account.amount,
        authority: transfer_authority_info.clone(),
        authority_signer_seeds: authority_signer_seeds,
        token_program: token_program_account_info.clone(),
    })?;

    spl_token_transfer(TokenTransferParams {
        source: governance_holding_account_info.clone(),
        destination: destination_governance_account_info.clone(),
        amount: total_reimbursement,
        authority: timelock_program_authority_info.clone(),
        authority_signer_seeds: authority_signer_seeds,
        token_program: token_program_account_info.clone(),
    })?;

    Ok(())
}
