//! Program state processor
use crate::{
    error::TimelockError,
    state::timelock_program::TimelockProgram,
    state::{enums::TimelockStateStatus, timelock_set::TimelockSet, timelock_state::TimelockState},
    utils::{
        assert_account_equiv, assert_draft, assert_initialized, assert_token_program_is_correct,
        spl_token_burn, TokenBurnParams,
    },
};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    entrypoint::ProgramResult,
    program_pack::Pack,
    pubkey::Pubkey,
    sysvar::Sysvar,
};
use spl_token::state::Mint;

/// Sign the set and say you're okay with moving it to voting stage.
pub fn process_sign(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let timelock_state_account_info = next_account_info(account_info_iter)?;
    let signatory_account_info = next_account_info(account_info_iter)?;
    let signatory_mint_info = next_account_info(account_info_iter)?;
    let timelock_set_account_info = next_account_info(account_info_iter)?;
    let transfer_authority_info = next_account_info(account_info_iter)?;
    let timelock_program_authority_info = next_account_info(account_info_iter)?;
    let timelock_program_account_info = next_account_info(account_info_iter)?;
    let token_program_account_info = next_account_info(account_info_iter)?;
    let clock_info = next_account_info(account_info_iter)?;

    let clock = Clock::from_account_info(clock_info)?;
    let mut timelock_state: TimelockState = assert_initialized(timelock_state_account_info)?;
    let timelock_set: TimelockSet = assert_initialized(timelock_set_account_info)?;
    let timelock_program: TimelockProgram = assert_initialized(timelock_program_account_info)?;
    let sig_mint: Mint = assert_initialized(signatory_mint_info)?;
    assert_token_program_is_correct(&timelock_program, token_program_account_info)?;
    assert_account_equiv(signatory_mint_info, &timelock_set.signatory_mint)?;
    assert_account_equiv(timelock_state_account_info, &timelock_set.state)?;
    assert_draft(&timelock_state)?;

    let (authority_key, bump_seed) =
        Pubkey::find_program_address(&[timelock_program_account_info.key.as_ref()], program_id);
    if timelock_program_authority_info.key != &authority_key {
        return Err(TimelockError::InvalidTimelockAuthority.into());
    }
    let authority_signer_seeds = &[timelock_program_account_info.key.as_ref(), &[bump_seed]];
    // the act of burning / signing is itself an assertion of permission...
    // if you lack the ability to do this, you lack permission to do it. no need to assert permission before
    // trying here.
    spl_token_burn(TokenBurnParams {
        mint: signatory_mint_info.clone(),
        amount: 1,
        authority: transfer_authority_info.clone(),
        authority_signer_seeds: authority_signer_seeds,
        token_program: token_program_account_info.clone(),
        source: signatory_account_info.clone(),
    })?;

    // assuming sig_mint object is now out of date, sub 1
    if sig_mint.supply - 1 == 0 {
        timelock_state.status = TimelockStateStatus::Voting;
        timelock_state.voting_began_at = clock.slot;

        TimelockState::pack(
            timelock_state.clone(),
            &mut timelock_state_account_info.data.borrow_mut(),
        )?;
    }

    Ok(())
}
