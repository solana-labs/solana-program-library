//! Program state processor

use crate::{
    error::TimelockError,
    state::{
        timelock_program::TimelockProgram,
        timelock_set::{TimelockSet, TIMELOCK_SET_VERSION},
    },
    utils::{
        assert_initialized, assert_rent_exempt, assert_uninitialized, spl_token_init_mint,
        spl_token_mint_to, TokenInitializeMintParams, TokenMintToParams,
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

/// Create a new timelock program
pub fn process_init_timelock_set<'a>(
    program_id: &'a Pubkey,
    accounts: &'a [AccountInfo<'a>],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let timelock_set_account_info = next_account_info(account_info_iter)?;
    let signatory_mint_account_info = next_account_info(account_info_iter)?;
    let admin_mint_account_info = next_account_info(account_info_iter)?;
    let voting_mint_account_info = next_account_info(account_info_iter)?;
    let destination_account_info = next_account_info(account_info_iter)?;
    let timelock_program_info = next_account_info(account_info_iter)?;

    let rent_info = next_account_info(account_info_iter)?;
    let rent = &Rent::from_account_info(rent_info)?;
    let token_program_info = next_account_info(account_info_iter)?;

    let timelock_program: TimelockProgram = assert_initialized(timelock_program_info)?;
    if &timelock_program.token_program_id != token_program_info.key {
        return Err(TimelockError::InvalidTokenProgram.into());
    };

    assert_rent_exempt(rent, timelock_set_account_info)?;

    let mut new_timelock_set: TimelockSet = assert_uninitialized(timelock_set_account_info)?;
    new_timelock_set.version = TIMELOCK_SET_VERSION;

    // now create the mints.

    new_timelock_set.admin_mint = *admin_mint_account_info.key;
    new_timelock_set.voting_mint = *voting_mint_account_info.key;
    new_timelock_set.signatory_mint = *signatory_mint_account_info.key;

    assert_rent_exempt(rent, admin_mint_account_info)?;
    assert_rent_exempt(rent, voting_mint_account_info)?;
    assert_rent_exempt(rent, signatory_mint_account_info)?;

    let _admin_mint: Mint = assert_initialized(admin_mint_account_info)?;
    let _voting_mint: Mint = assert_initialized(voting_mint_account_info)?;
    let _signatory_mint: Mint = assert_initialized(signatory_mint_account_info)?;

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
        Pubkey::find_program_address(&[timelock_program_info.key.as_ref()], program_id);

    let authority_signer_seeds = &[timelock_program_info.key.as_ref(), &[bump_seed]];

    // Mint admin token to creator
    spl_token_mint_to(TokenMintToParams {
        mint: admin_mint_account_info.clone(),
        destination: destination_account_info.clone(),
        amount: 1,
        authority: timelock_program_info.clone(),
        authority_signer_seeds: authority_signer_seeds,
        token_program: token_program_info.clone(),
    })?;

    // Mint signatory token to creator
    spl_token_mint_to(TokenMintToParams {
        mint: signatory_mint_account_info.clone(),
        destination: destination_account_info.clone(),
        amount: 1,
        authority: timelock_program_info.clone(),
        authority_signer_seeds: authority_signer_seeds,
        token_program: token_program_info.clone(),
    })?;
    Ok(())
}
