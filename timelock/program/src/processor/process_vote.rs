//! Program state processor
use crate::{
    state::timelock_program::TimelockProgram,
    state::{enums::TimelockStateStatus, timelock_set::TimelockSet},
    utils::{
        assert_initialized, assert_is_permissioned, assert_same_version_as_program, assert_voting,
        spl_token_burn, TokenBurnParams,
    },
};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    program_pack::Pack,
    pubkey::Pubkey,
};
use spl_token::state::Mint;

/// Vote on the timelock
pub fn process_vote(
    _: &Pubkey,
    accounts: &[AccountInfo],
    voting_token_amount: u64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let timelock_set_account_info = next_account_info(account_info_iter)?;
    let voting_account_info = next_account_info(account_info_iter)?;
    let voting_mint_account_info = next_account_info(account_info_iter)?;
    let voting_validation_account_info = next_account_info(account_info_iter)?;
    let timelock_program_account_info = next_account_info(account_info_iter)?;
    let token_program_account_info = next_account_info(account_info_iter)?;
    let mut timelock_set: TimelockSet = assert_initialized(timelock_set_account_info)?;
    let timelock_program: TimelockProgram = assert_initialized(timelock_program_account_info)?;

    assert_same_version_as_program(&timelock_program, &timelock_set)?;
    assert_voting(&timelock_set)?;
    assert_is_permissioned(
        voting_account_info,
        voting_validation_account_info,
        timelock_program_account_info,
        token_program_account_info,
    )?;
    let (_, bump_seed) = Pubkey::find_program_address(
        &[timelock_set_account_info.key.as_ref()],
        timelock_program_account_info.key,
    );

    let authority_signer_seeds = &[token_program_account_info.key.as_ref(), &[bump_seed]];
    let mint: Mint = assert_initialized(voting_mint_account_info)?;
    let now_remaining = mint.supply - voting_token_amount;
    let total_ever_existed = timelock_set.state.total_voting_tokens_minted;

    spl_token_burn(TokenBurnParams {
        mint: voting_mint_account_info.clone(),
        amount: voting_token_amount,
        authority: timelock_program_account_info.clone(),
        authority_signer_seeds: authority_signer_seeds,
        token_program: token_program_account_info.clone(),
        source: voting_account_info.clone(),
    })?;

    let tipped: bool = match timelock_set.config.consensus_algorithm {
        crate::state::enums::ConsensusAlgorithm::Majority => {
            (now_remaining as f64 / total_ever_existed as f64) < 0.5
        }

        crate::state::enums::ConsensusAlgorithm::SuperMajority => {
            (now_remaining as f64 / total_ever_existed as f64) < 0.66
        }

        crate::state::enums::ConsensusAlgorithm::FullConsensus => now_remaining == 0,
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
