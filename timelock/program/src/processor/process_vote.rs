//! Program state processor
use crate::{
    error::TimelockError,
    state::{
        enums::TimelockStateStatus, timelock_config::TimelockConfig, timelock_set::TimelockSet,
        timelock_state::TimelockState,
    },
    utils::{
        assert_account_equiv, assert_initialized, assert_voting, pull_mint_supply, spl_token_burn,
        spl_token_mint_to, TokenBurnParams, TokenMintToParams,
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

/// Vote on the timelock
pub fn process_vote(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    yes_voting_token_amount: u64,
    no_voting_token_amount: u64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let timelock_state_account_info = next_account_info(account_info_iter)?;
    let voting_account_info = next_account_info(account_info_iter)?;
    let yes_voting_account_info = next_account_info(account_info_iter)?;
    let no_voting_account_info = next_account_info(account_info_iter)?;
    let voting_mint_account_info = next_account_info(account_info_iter)?;
    let yes_voting_mint_account_info = next_account_info(account_info_iter)?;
    let no_voting_mint_account_info = next_account_info(account_info_iter)?;
    let source_mint_account_info = next_account_info(account_info_iter)?;
    let timelock_set_account_info = next_account_info(account_info_iter)?;
    let timelock_config_account_info = next_account_info(account_info_iter)?;
    let transfer_authority_info = next_account_info(account_info_iter)?;
    let timelock_program_authority_info = next_account_info(account_info_iter)?;
    let timelock_program_account_info = next_account_info(account_info_iter)?;
    let token_program_account_info = next_account_info(account_info_iter)?;
    let clock_info = next_account_info(account_info_iter)?;

    let clock = Clock::from_account_info(clock_info)?;
    let mut timelock_state: TimelockState = assert_initialized(timelock_state_account_info)?;
    let timelock_set: TimelockSet = assert_initialized(timelock_set_account_info)?;
    let timelock_config: TimelockConfig = assert_initialized(timelock_config_account_info)?;

    assert_account_equiv(voting_mint_account_info, &timelock_set.voting_mint)?;
    assert_account_equiv(yes_voting_mint_account_info, &timelock_set.yes_voting_mint)?;
    assert_account_equiv(no_voting_mint_account_info, &timelock_set.no_voting_mint)?;
    assert_account_equiv(timelock_config_account_info, &timelock_set.config)?;
    assert_account_equiv(timelock_state_account_info, &timelock_set.state)?;
    assert_account_equiv(source_mint_account_info, &timelock_set.source_mint)?;

    assert_voting(&timelock_state)?;

    let (authority_key, bump_seed) =
        Pubkey::find_program_address(&[timelock_program_account_info.key.as_ref()], program_id);
    if timelock_program_authority_info.key != &authority_key {
        return Err(TimelockError::InvalidTimelockAuthority.into());
    }
    let authority_signer_seeds = &[timelock_program_account_info.key.as_ref(), &[bump_seed]];

    // We dont initialize the mints because it's too expensive on the stack size.
    let source_mint_supply: u64 = pull_mint_supply(source_mint_account_info)?;
    let yes_mint_supply: u64 = pull_mint_supply(yes_voting_mint_account_info)?;

    let total_ever_existed = source_mint_supply;

    let mut now_remaining_in_no_column =
        match source_mint_supply.checked_sub(yes_voting_token_amount) {
            Some(val) => val,
            None => return Err(TimelockError::NumericalOverflow.into()),
        };

    now_remaining_in_no_column = match now_remaining_in_no_column.checked_sub(yes_mint_supply) {
        Some(val) => val,
        None => return Err(TimelockError::NumericalOverflow.into()),
    };

    // The act of voting proves you are able to vote. No need to assert permission here.
    spl_token_burn(TokenBurnParams {
        mint: voting_mint_account_info.clone(),
        amount: yes_voting_token_amount + no_voting_token_amount,
        authority: transfer_authority_info.clone(),
        authority_signer_seeds: authority_signer_seeds,
        token_program: token_program_account_info.clone(),
        source: voting_account_info.clone(),
    })?;

    spl_token_mint_to(TokenMintToParams {
        mint: yes_voting_mint_account_info.clone(),
        destination: yes_voting_account_info.clone(),
        amount: yes_voting_token_amount,
        authority: timelock_program_authority_info.clone(),
        authority_signer_seeds: authority_signer_seeds,
        token_program: token_program_account_info.clone(),
    })?;

    spl_token_mint_to(TokenMintToParams {
        mint: no_voting_mint_account_info.clone(),
        destination: no_voting_account_info.clone(),
        amount: no_voting_token_amount,
        authority: timelock_program_authority_info.clone(),
        authority_signer_seeds: authority_signer_seeds,
        token_program: token_program_account_info.clone(),
    })?;

    let tipped: bool = match timelock_config.consensus_algorithm {
        crate::state::enums::ConsensusAlgorithm::Majority => {
            (now_remaining_in_no_column as f64 / total_ever_existed as f64) < 0.5
        }

        crate::state::enums::ConsensusAlgorithm::SuperMajority => {
            (now_remaining_in_no_column as f64 / total_ever_existed as f64) < 0.66
        }

        crate::state::enums::ConsensusAlgorithm::FullConsensus => now_remaining_in_no_column == 0,
    };

    let elapsed = match clock.slot.checked_sub(timelock_state.voting_began_at) {
        Some(val) => val,
        None => return Err(TimelockError::NumericalOverflow.into()),
    };
    let too_long = elapsed > timelock_config.time_limit;

    if tipped || too_long {
        if tipped {
            timelock_state.status = TimelockStateStatus::Executing;
        } else {
            timelock_state.status = TimelockStateStatus::Defeated;
        }
        timelock_state.voting_ended_at = clock.slot;

        TimelockState::pack(
            timelock_state.clone(),
            &mut timelock_state_account_info.data.borrow_mut(),
        )?;
    }

    Ok(())
}
