//! Program state processor
use std::mem::uninitialized;

use crate::{
    state::timelock_program::TimelockProgram,
    state::{enums::TimelockStateStatus, timelock_set::TimelockSet},
    utils::{
        assert_draft, assert_initialized, assert_is_permissioned, assert_same_version_as_program,
        assert_uninitialized, assert_voting, spl_token_burn, spl_token_init_account,
        spl_token_mint_to, TokenBurnParams, TokenInitializeAccountParams, TokenMintToParams,
    },
};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    program_pack::Pack,
    pubkey::Pubkey,
};

/// Mint voting tokens
pub fn process_mint_voting_tokens(
    _: &Pubkey,
    accounts: &[AccountInfo],
    voting_token_amount: u64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let timelock_set_account_info = next_account_info(account_info_iter)?;
    let voting_account_info = next_account_info(account_info_iter)?;
    let voting_mint_account_info = next_account_info(account_info_iter)?;
    let signatory_account_info = next_account_info(account_info_iter)?;
    let signatory_validation_account_info = next_account_info(account_info_iter)?;
    let timelock_program_account_info = next_account_info(account_info_iter)?;
    let token_program_account_info = next_account_info(account_info_iter)?;
    let rent_info = next_account_info(account_info_iter)?;

    let mut timelock_set: TimelockSet = assert_initialized(timelock_set_account_info)?;
    let timelock_program: TimelockProgram = assert_initialized(timelock_program_account_info)?;

    assert_same_version_as_program(&timelock_program, &timelock_set)?;
    assert_draft(&timelock_set)?;
    assert_is_permissioned(
        signatory_account_info,
        signatory_validation_account_info,
        timelock_program_account_info,
        token_program_account_info,
    )?;

    if voting_account_info.data_is_empty() {
        spl_token_init_account(TokenInitializeAccountParams {
            account: voting_account_info.clone(),
            mint: voting_mint_account_info.clone(),
            owner: timelock_program_account_info.clone(),
            rent: rent_info.clone(),
            token_program: token_program_account_info.clone(),
        })?
    }

    let (_, bump_seed) = Pubkey::find_program_address(
        &[timelock_set_account_info.key.as_ref()],
        timelock_program_account_info.key,
    );

    let authority_signer_seeds = &[token_program_account_info.key.as_ref(), &[bump_seed]];

    spl_token_mint_to(TokenMintToParams {
        mint: voting_mint_account_info.clone(),
        destination: voting_account_info.clone(),
        amount: voting_token_amount,
        authority: timelock_program_account_info.clone(),
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
