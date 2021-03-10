//! Program state processor

use crate::{
    error::TimelockError,
    state::timelock_program::TimelockProgram,
    state::timelock_set::TimelockSet,
    utils::{
        assert_draft, assert_initialized, assert_is_permissioned, assert_same_version_as_program,
        spl_token_mint_to, TokenMintToParams,
    },
};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    program_pack::Pack,
    pubkey::Pubkey,
};
use spl_token::state::{Account, Mint};

/// Mint voting tokens
pub fn process_mint_voting_tokens(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    voting_token_amount: u64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let timelock_set_account_info = next_account_info(account_info_iter)?;
    let voting_account_info = next_account_info(account_info_iter)?;
    let voting_mint_account_info = next_account_info(account_info_iter)?;
    let signatory_account_info = next_account_info(account_info_iter)?;
    let signatory_validation_account_info = next_account_info(account_info_iter)?;
    let transfer_authority_info = next_account_info(account_info_iter)?;
    let timelock_program_authority_info = next_account_info(account_info_iter)?;
    let timelock_program_account_info = next_account_info(account_info_iter)?;
    let token_program_account_info = next_account_info(account_info_iter)?;

    let mut timelock_set: TimelockSet = assert_initialized(timelock_set_account_info)?;
    let timelock_program: TimelockProgram = assert_initialized(timelock_program_account_info)?;

    if voting_token_amount < 0 as u64  {
        return Err(TimelockError::TokenAmountBelowZero.into());
    }

    assert_same_version_as_program(&timelock_program, &timelock_set)?;
    assert_draft(&timelock_set)?;
    assert_is_permissioned(
        program_id,
        signatory_account_info,
        signatory_validation_account_info,
        timelock_program_account_info,
        token_program_account_info,
        transfer_authority_info,
        timelock_program_authority_info,
    )?;
    let _voting_account: Account = assert_initialized(voting_account_info)?;
    let _voting_mint: Mint = assert_initialized(voting_mint_account_info)?;

    let (authority_key, bump_seed) =
        Pubkey::find_program_address(&[timelock_program_account_info.key.as_ref()], program_id);
    if timelock_program_authority_info.key != &authority_key {
        return Err(TimelockError::InvalidTimelockAuthority.into());
    }
    let authority_signer_seeds = &[timelock_program_account_info.key.as_ref(), &[bump_seed]];

    spl_token_mint_to(TokenMintToParams {
        mint: voting_mint_account_info.clone(),
        destination: voting_account_info.clone(),
        amount: voting_token_amount,
        authority: timelock_program_authority_info.clone(),
        authority_signer_seeds: authority_signer_seeds,
        token_program: token_program_account_info.clone(),
    })?;
    timelock_set.state.total_voting_tokens_minted += voting_token_amount;

    TimelockSet::pack(
        timelock_set.clone(),
        &mut timelock_set_account_info.data.borrow_mut(),
    )?;

    Ok(())
}
