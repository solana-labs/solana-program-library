//! Program state processor
use crate::{
    error::TimelockError,
    state::enums::{ExecutionType, GovernanceAccountType, TimelockType, VotingEntryRule},
    state::governance::{Governance, CONFIG_NAME_LENGTH},
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
pub fn process_init_governance(
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
    let governance_account_info = next_account_info(account_info_iter)?;
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
    if governance_account_info.key != &config_key {
        return Err(TimelockError::InvalidGovernanceKey.into());
    }
    let mut new_governance: Governance = assert_uninitialized(governance_account_info)?;
    new_governance.account_type = GovernanceAccountType::Governance;
    new_governance.name = name;
    new_governance.minimum_slot_waiting_period = minimum_slot_waiting_period;
    new_governance.time_limit = time_limit;
    new_governance.program = *program_to_tie_account_info.key;
    new_governance.governance_mint = *governance_mint_account_info.key;

    new_governance.council_mint = council_mint;

    new_governance.vote_threshold = vote_threshold;
    new_governance.execution_type = match execution_type {
        0 => ExecutionType::Independent,
        _ => ExecutionType::Independent,
    };

    new_governance.timelock_type = match timelock_type {
        0 => TimelockType::Governance,
        _ => TimelockType::Governance,
    };

    new_governance.voting_entry_rule = match voting_entry_rule {
        0 => VotingEntryRule::Anytime,
        _ => VotingEntryRule::Anytime,
    };

    Governance::pack(
        new_governance,
        &mut governance_account_info.data.borrow_mut(),
    )?;

    Ok(())
}
