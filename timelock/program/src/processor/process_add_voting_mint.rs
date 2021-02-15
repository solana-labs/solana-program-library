//! Program state processor

use crate::{
    state::{timelock_program::TimelockProgram, timelock_set::TimelockSet},
    utils::{
        assert_draft, assert_initialized, assert_is_permissioned, assert_rent_exempt,
        assert_same_version_as_program, assert_token_program_is_correct, assert_uninitialized,
        spl_token_init_account, spl_token_init_mint, TokenInitializeAccountParams,
        TokenInitializeMintParams,
    },
};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    program_pack::Pack,
    pubkey::Pubkey,
    sysvar::{rent::Rent, Sysvar},
};
use spl_token::state::Mint;

/// Create a new timelock set
pub fn process_add_voting_mint(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let timelock_set_account_info = next_account_info(account_info_iter)?;
    let admin_account_info = next_account_info(account_info_iter)?;
    let admin_validation_account_info = next_account_info(account_info_iter)?;
    let voting_mint_account_info = next_account_info(account_info_iter)?;
    let voting_validation_account_info = next_account_info(account_info_iter)?;
    let timelock_program_info = next_account_info(account_info_iter)?;
    let token_program_info = next_account_info(account_info_iter)?;
    let rent_info = next_account_info(account_info_iter)?;
    let rent = &Rent::from_account_info(rent_info)?;

    let timelock_program: TimelockProgram = assert_initialized(timelock_program_info)?;

    let mut timelock_set: TimelockSet = assert_initialized(timelock_set_account_info)?;
    let timelock_program: TimelockProgram = assert_initialized(timelock_program_info)?;

    assert_same_version_as_program(&timelock_program, &timelock_set)?;
    assert_token_program_is_correct(&timelock_program, token_program_info)?;
    assert_draft(&timelock_set)?;
    assert_is_permissioned(
        admin_account_info,
        admin_validation_account_info,
        timelock_program_info,
        token_program_info,
    )?;

    // now create the mints.

    timelock_set.voting_mint = *voting_mint_account_info.key;

    timelock_set.voting_validation = *voting_validation_account_info.key;

    assert_rent_exempt(rent, voting_mint_account_info)?;

    let _voting_mint: Mint = assert_uninitialized(voting_mint_account_info)?;

    TimelockSet::pack(
        timelock_set.clone(),
        &mut timelock_set_account_info.data.borrow_mut(),
    )?;

    // Initialize mint
    spl_token_init_mint(TokenInitializeMintParams {
        mint: voting_mint_account_info.clone(),
        authority: program_id,
        rent: rent_info.clone(),
        decimals: 8,
        token_program: token_program_info.clone(),
    })?;

    // Initialize validation accounts
    spl_token_init_account(TokenInitializeAccountParams {
        account: voting_validation_account_info.clone(),
        mint: voting_mint_account_info.clone(),
        owner: timelock_program_info.clone(),
        rent: rent_info.clone(),
        token_program: token_program_info.clone(),
    })?;

    Ok(())
}
