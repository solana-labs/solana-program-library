//! Program state processor
use crate::{error::TimelockError, state::timelock_program::TimelockProgram, state::{enums::TimelockStateStatus, timelock_set::TimelockSet}, utils::{TokenBurnParams, TokenMintToParams, assert_initialized, assert_is_permissioned, assert_same_version_as_program, assert_voting, spl_token_burn, spl_token_mint_to}};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    program_pack::Pack,
    pubkey::Pubkey,
};
use spl_token::state::Mint;

/// Vote on the timelock
pub fn process_vote(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    yes_voting_token_amount: u64,
    no_voting_token_amount: u64
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let timelock_set_account_info = next_account_info(account_info_iter)?;
    let voting_account_info = next_account_info(account_info_iter)?;
    let yes_voting_account_info = next_account_info(account_info_iter)?;
    let no_voting_account_info = next_account_info(account_info_iter)?;
    let voting_mint_account_info = next_account_info(account_info_iter)?;
    let yes_voting_mint_account_info = next_account_info(account_info_iter)?;
    let no_voting_mint_account_info = next_account_info(account_info_iter)?;
    let transfer_authority_info = next_account_info(account_info_iter)?;
    let timelock_program_authority_info = next_account_info(account_info_iter)?;
    let timelock_program_account_info = next_account_info(account_info_iter)?;
    let token_program_account_info = next_account_info(account_info_iter)?;
    let mut timelock_set: TimelockSet = assert_initialized(timelock_set_account_info)?;
    let timelock_program: TimelockProgram = assert_initialized(timelock_program_account_info)?;

    assert_same_version_as_program(&timelock_program, &timelock_set)?;
    assert_voting(&timelock_set)?;

    let (authority_key, bump_seed) =
        Pubkey::find_program_address(&[timelock_program_account_info.key.as_ref()], program_id);
    if timelock_program_authority_info.key != &authority_key {
        return Err(TimelockError::InvalidTimelockAuthority.into());
    }
    let authority_signer_seeds = &[timelock_program_account_info.key.as_ref(), &[bump_seed]];

    let mint: Mint = assert_initialized(voting_mint_account_info)?;
    let yes_mint: Mint = assert_initialized(yes_voting_mint_account_info)?;
    let no_mint: Mint = assert_initialized(no_voting_mint_account_info)?;

    let now_remaining_in_no_column = mint.supply + no_voting_token_amount - yes_voting_token_amount;
    let total_ever_existed = mint.supply + yes_mint.supply + no_mint.supply;
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


    let tipped: bool = match timelock_set.config.consensus_algorithm {
        crate::state::enums::ConsensusAlgorithm::Majority => {
            (now_remaining_in_no_column as f64 / total_ever_existed as f64) < 0.5
        }

        crate::state::enums::ConsensusAlgorithm::SuperMajority => {
            (now_remaining_in_no_column as f64 / total_ever_existed as f64) < 0.66
        }

        crate::state::enums::ConsensusAlgorithm::FullConsensus => now_remaining_in_no_column == 0,
    };

    if tipped {
        timelock_set.state.status = TimelockStateStatus::Executing;

        TimelockSet::pack(
            timelock_set.clone(),
            &mut timelock_set_account_info.data.borrow_mut(),
        )?;
    }

    Ok(())
}
