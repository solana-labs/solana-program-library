//! Program state processor
use crate::{
    error::TimelockError,
    state::enums::{ExecutionType, GovernanceAccountType, TimelockType, VotingEntryRule},
    state::timelock_config::{TimelockConfig, CONFIG_NAME_LENGTH},
    utils::assert_uninitialized,
    PROGRAM_AUTHORITY_SEED,
};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    program_pack::Pack,
    pubkey::Pubkey,
};

/// Init timelock config
#[allow(clippy::too_many_arguments)]
pub fn process_init_timelock_config(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    vote_threshold: u8,
    execution_type: u8,
    timelock_type: u8,
    voting_entry_rule: u8,
    minimum_slot_waiting_period: u64,
    time_limit: u64,
    name: [u8; CONFIG_NAME_LENGTH],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let timelock_config_account_info = next_account_info(account_info_iter)?;
    let program_to_tie_account_info = next_account_info(account_info_iter)?;
    let governance_mint_account_info = next_account_info(account_info_iter)?;

    let (council_mint, council_mint_seed) = next_account_info(account_info_iter)
        .map(|acc| (Some(*acc.key), acc.key.as_ref()))
        .unwrap_or((None, &[]));

    let seeds = &[
        PROGRAM_AUTHORITY_SEED,
        program_id.as_ref(),
        governance_mint_account_info.key.as_ref(),
        council_mint_seed,
        program_to_tie_account_info.key.as_ref(),
    ];
    let (config_key, _) = Pubkey::find_program_address(seeds, program_id);
    if timelock_config_account_info.key != &config_key {
        return Err(TimelockError::InvalidTimelockConfigKey.into());
    }
    let mut new_timelock_config: TimelockConfig =
        assert_uninitialized(timelock_config_account_info)?;
    new_timelock_config.account_type = GovernanceAccountType::Governance;
    new_timelock_config.name = name;
    new_timelock_config.minimum_slot_waiting_period = minimum_slot_waiting_period;
    new_timelock_config.time_limit = time_limit;
    new_timelock_config.program = *program_to_tie_account_info.key;
    new_timelock_config.governance_mint = *governance_mint_account_info.key;

    new_timelock_config.council_mint = council_mint;

    new_timelock_config.vote_threshold = vote_threshold;
    new_timelock_config.execution_type = match execution_type {
        0 => ExecutionType::Independent,
        _ => ExecutionType::Independent,
    };

    new_timelock_config.timelock_type = match timelock_type {
        0 => TimelockType::Governance,
        _ => TimelockType::Governance,
    };

    new_timelock_config.voting_entry_rule = match voting_entry_rule {
        0 => VotingEntryRule::Anytime,
        _ => VotingEntryRule::Anytime,
    };

    TimelockConfig::pack(
        new_timelock_config,
        &mut timelock_config_account_info.data.borrow_mut(),
    )?;

    Ok(())
}
