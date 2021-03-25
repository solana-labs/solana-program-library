//! Program state processor
use crate::{
    error::TimelockError,
    state::timelock_config::{TimelockConfig, CONFIG_NAME_LENGTH, TIMELOCK_CONFIG_VERSION},
    state::{
        enums::{ConsensusAlgorithm, ExecutionType, TimelockType, VotingEntryRule},
        timelock_program::TimelockProgram,
    },
    utils::{assert_initialized, assert_token_program_is_correct, assert_uninitialized},
};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    program_pack::Pack,
    pubkey::Pubkey,
};

/// Init timelock config
pub fn process_init_timelock_config(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    consensus_algorithm: u8,
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
    let council_mint_account_info = next_account_info(account_info_iter)?;
    let timelock_program_account_info = next_account_info(account_info_iter)?;
    let token_program_account_info = next_account_info(account_info_iter)?;

    let timelock_program: TimelockProgram = assert_initialized(timelock_program_account_info)?;

    assert_token_program_is_correct(&timelock_program, token_program_account_info)?;
    let seeds = &[
        timelock_program_account_info.key.as_ref(),
        governance_mint_account_info.key.as_ref(),
        council_mint_account_info.key.as_ref(),
        program_to_tie_account_info.key.as_ref(),
    ];
    let (config_key, _) = Pubkey::find_program_address(seeds, program_id);
    if timelock_config_account_info.key != &config_key {
        return Err(TimelockError::InvalidTimelockConfigKey.into());
    }
    let mut new_timelock_config: TimelockConfig =
        assert_uninitialized(timelock_config_account_info)?;
    new_timelock_config.version = TIMELOCK_CONFIG_VERSION;
    new_timelock_config.name = name;
    new_timelock_config.minimum_slot_waiting_period = minimum_slot_waiting_period;
    new_timelock_config.time_limit = time_limit;
    new_timelock_config.program = *program_to_tie_account_info.key;
    new_timelock_config.governance_mint = *governance_mint_account_info.key;
    new_timelock_config.council_mint = *council_mint_account_info.key;
    new_timelock_config.consensus_algorithm = match consensus_algorithm {
        0 => ConsensusAlgorithm::Majority,
        1 => ConsensusAlgorithm::SuperMajority,
        2 => ConsensusAlgorithm::FullConsensus,
        _ => ConsensusAlgorithm::Majority,
    };
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
