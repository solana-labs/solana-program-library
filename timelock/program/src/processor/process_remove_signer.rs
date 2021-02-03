//! Program state processor
use crate::{
    state::timelock_program::TimelockProgram,
    state::timelock_set::TimelockSet,
    utils::{
        assert_draft, assert_initialized, assert_is_admin, assert_same_version_as_program,
        assert_token_program_is_correct, spl_token_burn, TokenBurnParams,
    },
};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    pubkey::Pubkey,
};

/// Removes a signer
pub fn process_remove_signer(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let remove_signatory_account_info = next_account_info(account_info_iter)?;
    let signatory_mint_info = next_account_info(account_info_iter)?;
    let admin_account_info = next_account_info(account_info_iter)?;
    let admin_validation_account_info = next_account_info(account_info_iter)?;
    let timelock_set_account_info = next_account_info(account_info_iter)?;
    let timelock_program_account_info = next_account_info(account_info_iter)?;
    let token_program_account_info = next_account_info(account_info_iter)?;

    let timelock_set: TimelockSet = assert_initialized(timelock_set_account_info)?;
    let timelock_program: TimelockProgram = assert_initialized(timelock_program_account_info)?;
    assert_same_version_as_program(&timelock_program, &timelock_set)?;
    assert_token_program_is_correct(&timelock_program, token_program_account_info)?;
    assert_draft(&timelock_set)?;
    assert_is_admin(
        admin_account_info,
        admin_validation_account_info,
        timelock_program_account_info,
        token_program_account_info,
    )?;
    let (_, bump_seed) =
        Pubkey::find_program_address(&[timelock_set_account_info.key.as_ref()], program_id);

    let authority_signer_seeds = &[token_program_account_info.key.as_ref(), &[bump_seed]];

    // Remove the token
    spl_token_burn(TokenBurnParams {
        mint: signatory_mint_info.clone(),
        amount: 1,
        authority: timelock_program_account_info.clone(),
        authority_signer_seeds: authority_signer_seeds,
        token_program: token_program_account_info.clone(),
        source: remove_signatory_account_info.clone(),
    })?;
    Ok(())
}
