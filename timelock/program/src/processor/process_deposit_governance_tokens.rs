//! Program state processor

use crate::{
    error::TimelockError,
    state::timelock_program::TimelockProgram,
    state::{
        enums::VotingEntryRule, timelock_config::TimelockConfig, timelock_set::TimelockSet,
        timelock_state::TimelockState,
    },
    utils::{
        assert_account_equiv, assert_draft, assert_initialized, assert_token_program_is_correct,
        spl_token_mint_to, spl_token_transfer, TokenMintToParams, TokenTransferParams,
    },
};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    pubkey::Pubkey,
};

/// Deposit governance tokens
pub fn process_deposit_governance_tokens(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    voting_token_amount: u64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let voting_account_info = next_account_info(account_info_iter)?;
    let source_governance_account_info = next_account_info(account_info_iter)?;
    let governance_holding_account_info = next_account_info(account_info_iter)?;
    let voting_mint_account_info = next_account_info(account_info_iter)?;
    let timelock_state_account_info = next_account_info(account_info_iter)?;
    let timelock_set_account_info = next_account_info(account_info_iter)?;
    let timelock_config_account_info = next_account_info(account_info_iter)?;
    let transfer_authority_info = next_account_info(account_info_iter)?;
    let timelock_program_authority_info = next_account_info(account_info_iter)?;
    let timelock_program_account_info = next_account_info(account_info_iter)?;
    let token_program_account_info = next_account_info(account_info_iter)?;

    let timelock_state: TimelockState = assert_initialized(timelock_state_account_info)?;
    let timelock_set: TimelockSet = assert_initialized(timelock_set_account_info)?;
    let timelock_program: TimelockProgram = assert_initialized(timelock_program_account_info)?;
    let timelock_config: TimelockConfig = assert_initialized(timelock_config_account_info)?;
    assert_token_program_is_correct(&timelock_program, token_program_account_info)?;

    assert_account_equiv(timelock_state_account_info, &timelock_set.state)?;
    assert_account_equiv(
        governance_holding_account_info,
        &timelock_set.governance_holding,
    )?;
    assert_account_equiv(voting_mint_account_info, &timelock_set.voting_mint)?;
    assert_account_equiv(timelock_config_account_info, &timelock_set.config)?;

    if voting_token_amount < 0 as u64 {
        return Err(TimelockError::TokenAmountBelowZero.into());
    }

    if timelock_config.voting_entry_rule == VotingEntryRule::DraftOnly {
        assert_draft(&timelock_state)?;
    }

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

    spl_token_transfer(TokenTransferParams {
        source: source_governance_account_info.clone(),
        destination: governance_holding_account_info.clone(),
        amount: voting_token_amount,
        authority: transfer_authority_info.clone(),
        authority_signer_seeds: authority_signer_seeds,
        token_program: token_program_account_info.clone(),
    })?;

    Ok(())
}
