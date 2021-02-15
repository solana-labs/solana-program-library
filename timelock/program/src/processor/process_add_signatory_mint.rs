//! Program state processor

use crate::{
    state::{timelock_program::TimelockProgram, timelock_set::TimelockSet},
    utils::{
        assert_draft, assert_initialized, assert_is_permissioned, assert_rent_exempt,
        assert_same_version_as_program, assert_token_program_is_correct, assert_uninitialized,
        spl_token_init_account, spl_token_init_mint, spl_token_mint_to,
        TokenInitializeAccountParams, TokenInitializeMintParams, TokenMintToParams,
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
pub fn process_add_signatory_mint(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let timelock_set_account_info = next_account_info(account_info_iter)?;
    let admin_account_info = next_account_info(account_info_iter)?;
    let admin_validation_account_info = next_account_info(account_info_iter)?;
    let signatory_mint_account_info = next_account_info(account_info_iter)?;
    let signatory_validation_account_info = next_account_info(account_info_iter)?;
    let destination_sig_account_info = next_account_info(account_info_iter)?;
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

    timelock_set.signatory_mint = *signatory_mint_account_info.key;

    timelock_set.signatory_validation = *signatory_validation_account_info.key;

    assert_rent_exempt(rent, signatory_mint_account_info)?;

    let _signatory_mint: Mint = assert_uninitialized(signatory_mint_account_info)?;

    TimelockSet::pack(
        timelock_set.clone(),
        &mut timelock_set_account_info.data.borrow_mut(),
    )?;

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

    Ok(())
}
