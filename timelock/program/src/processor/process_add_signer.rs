//! Program state processor
use crate::{error::TimelockError, state::timelock_program::TimelockProgram, state::timelock_set::TimelockSet, utils::{TokenMintToParams, assert_account_equiv, assert_draft, assert_initialized, assert_is_permissioned, assert_proper_signatory_mint, assert_same_version_as_program, assert_token_program_is_correct, spl_token_mint_to}};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    program_pack::Pack,
    pubkey::Pubkey,
};
use spl_token::state::{Account, Mint};

/// Adds a signer
pub fn process_add_signer(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let new_signatory_account_info = next_account_info(account_info_iter)?;
    let signatory_mint_info = next_account_info(account_info_iter)?;
    let admin_account_info = next_account_info(account_info_iter)?;
    let admin_validation_account_info = next_account_info(account_info_iter)?;
    let timelock_set_account_info = next_account_info(account_info_iter)?;
    let transfer_authority_info = next_account_info(account_info_iter)?;
    let timelock_program_authority_info = next_account_info(account_info_iter)?;
    let timelock_program_account_info = next_account_info(account_info_iter)?;
    let token_program_account_info = next_account_info(account_info_iter)?;

    let mut timelock_set: TimelockSet = assert_initialized(timelock_set_account_info)?;
    let timelock_program: TimelockProgram = assert_initialized(timelock_program_account_info)?;
    assert_account_equiv(admin_validation_account_info, &timelock_set.admin_validation)?;
    assert_same_version_as_program(&timelock_program, &timelock_set)?;
    assert_token_program_is_correct(&timelock_program, token_program_account_info)?;
    assert_proper_signatory_mint(&timelock_set, signatory_mint_info)?;
    assert_draft(&timelock_set)?;
    assert_is_permissioned(
        program_id,
        admin_account_info,
        admin_validation_account_info,
        timelock_program_account_info,
        token_program_account_info,
        transfer_authority_info,
        timelock_program_authority_info,
    )?;
    let _sig_account: Account = assert_initialized(new_signatory_account_info)?;
    let _sig_mint: Mint = assert_initialized(signatory_mint_info)?;

    let (authority_key, bump_seed) =
        Pubkey::find_program_address(&[timelock_program_account_info.key.as_ref()], program_id);
    if timelock_program_authority_info.key != &authority_key {
        return Err(TimelockError::InvalidTimelockAuthority.into());
    }
    let authority_signer_seeds = &[timelock_program_account_info.key.as_ref(), &[bump_seed]];

    spl_token_mint_to(TokenMintToParams {
        mint: signatory_mint_info.clone(),
        destination: new_signatory_account_info.clone(),
        amount: 1,
        authority: timelock_program_authority_info.clone(),
        authority_signer_seeds: authority_signer_seeds,
        token_program: token_program_account_info.clone(),
    })?;
    timelock_set.state.total_signing_tokens_minted += 1;

    TimelockSet::pack(
        timelock_set.clone(),
        &mut timelock_set_account_info.data.borrow_mut(),
    )?;
    Ok(())
}
