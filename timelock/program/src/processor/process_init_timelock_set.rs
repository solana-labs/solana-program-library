//! Program state processor

use crate::{
    state::{
        timelock_program::TimelockProgram,
        timelock_set::{TimelockSet, TIMELOCK_SET_VERSION},
    },
    utils::{
        assert_initialized, assert_rent_exempt, assert_same_version_as_program,
        assert_token_program_is_correct, assert_uninitialized, spl_token_init_account,
        spl_token_init_mint, spl_token_mint_to, TokenInitializeAccountParams,
        TokenInitializeMintParams, TokenMintToParams,
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
pub fn process_init_timelock_set<'a>(
    program_id: &'a Pubkey,
    accounts: &'a [AccountInfo<'a>],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let timelock_set_account_info = next_account_info(account_info_iter)?;
    let signatory_mint_account_info = next_account_info(account_info_iter)?;
    let admin_mint_account_info = next_account_info(account_info_iter)?;
    let voting_mint_account_info = next_account_info(account_info_iter)?;
    let signatory_validation_account_info = next_account_info(account_info_iter)?;
    let admin_validation_account_info = next_account_info(account_info_iter)?;
    let voting_validation_account_info = next_account_info(account_info_iter)?;
    let destination_admin_account_info = next_account_info(account_info_iter)?;
    let destination_sig_account_info = next_account_info(account_info_iter)?;
    let timelock_program_info = next_account_info(account_info_iter)?;
    let token_program_info = next_account_info(account_info_iter)?;
    let rent_info = next_account_info(account_info_iter)?;
    let rent = &Rent::from_account_info(rent_info)?;

    let timelock_program: TimelockProgram = assert_initialized(timelock_program_info)?;

    assert_rent_exempt(rent, timelock_set_account_info)?;

    let mut new_timelock_set: TimelockSet = assert_uninitialized(timelock_set_account_info)?;
    new_timelock_set.version = TIMELOCK_SET_VERSION;

    assert_same_version_as_program(&timelock_program, &new_timelock_set)?;
    assert_token_program_is_correct(&timelock_program, token_program_info)?;
    // now create the mints.

    new_timelock_set.admin_mint = *admin_mint_account_info.key;
    new_timelock_set.voting_mint = *voting_mint_account_info.key;
    new_timelock_set.signatory_mint = *signatory_mint_account_info.key;

    new_timelock_set.admin_validation = *admin_validation_account_info.key;
    new_timelock_set.voting_validation = *voting_validation_account_info.key;
    new_timelock_set.signatory_validation = *signatory_validation_account_info.key;

    assert_rent_exempt(rent, admin_mint_account_info)?;
    assert_rent_exempt(rent, voting_mint_account_info)?;
    assert_rent_exempt(rent, signatory_mint_account_info)?;

    let _admin_mint: Mint = assert_uninitialized(admin_mint_account_info)?;
    let _voting_mint: Mint = assert_uninitialized(voting_mint_account_info)?;
    let _signatory_mint: Mint = assert_uninitialized(signatory_mint_account_info)?;

    TimelockSet::pack(
        new_timelock_set.clone(),
        &mut timelock_set_account_info.data.borrow_mut(),
    )?;

    spl_token_init_mint(TokenInitializeMintParams {
        mint: admin_mint_account_info.clone(),
        authority: program_id,
        rent: rent_info.clone(),
        decimals: 8,
        token_program: token_program_info.clone(),
    })?;

    spl_token_init_mint(TokenInitializeMintParams {
        mint: voting_mint_account_info.clone(),
        authority: program_id,
        rent: rent_info.clone(),
        decimals: 8,
        token_program: token_program_info.clone(),
    })?;

    spl_token_init_mint(TokenInitializeMintParams {
        mint: signatory_mint_account_info.clone(),
        authority: program_id,
        rent: rent_info.clone(),
        decimals: 8,
        token_program: token_program_info.clone(),
    })?;
    let (_, bump_seed) =
        Pubkey::find_program_address(&[timelock_set_account_info.key.as_ref()], program_id);

    let authority_signer_seeds = &[timelock_program_info.key.as_ref(), &[bump_seed]];

    // Mint admin token to creator
    spl_token_init_account(TokenInitializeAccountParams {
        account: destination_admin_account_info.clone(),
        mint: admin_mint_account_info.clone(),
        owner: timelock_program_info.clone(),
        rent: rent_info.clone(),
        token_program: token_program_info.clone(),
    })?;

    spl_token_mint_to(TokenMintToParams {
        mint: admin_mint_account_info.clone(),
        destination: destination_admin_account_info.clone(),
        amount: 1,
        authority: timelock_program_info.clone(),
        authority_signer_seeds: authority_signer_seeds,
        token_program: token_program_info.clone(),
    })?;

    // Mint signatory token to creator
    spl_token_init_account(TokenInitializeAccountParams {
        account: destination_sig_account_info.clone(),
        mint: signatory_mint_account_info.clone(),
        owner: timelock_program_info.clone(),
        rent: rent_info.clone(),
        token_program: token_program_info.clone(),
    })?;

    spl_token_mint_to(TokenMintToParams {
        mint: signatory_mint_account_info.clone(),
        destination: destination_sig_account_info.clone(),
        amount: 1,
        authority: timelock_program_info.clone(),
        authority_signer_seeds: authority_signer_seeds,
        token_program: token_program_info.clone(),
    })?;

    // Initialize validation accounts
    spl_token_init_account(TokenInitializeAccountParams {
        account: signatory_validation_account_info.clone(),
        mint: signatory_mint_account_info.clone(),
        owner: timelock_program_info.clone(),
        rent: rent_info.clone(),
        token_program: token_program_info.clone(),
    })?;

    spl_token_init_account(TokenInitializeAccountParams {
        account: admin_validation_account_info.clone(),
        mint: admin_mint_account_info.clone(),
        owner: timelock_program_info.clone(),
        rent: rent_info.clone(),
        token_program: token_program_info.clone(),
    })?;

    spl_token_init_account(TokenInitializeAccountParams {
        account: voting_validation_account_info.clone(),
        mint: voting_mint_account_info.clone(),
        owner: timelock_program_info.clone(),
        rent: rent_info.clone(),
        token_program: token_program_info.clone(),
    })?;
    Ok(())
}
