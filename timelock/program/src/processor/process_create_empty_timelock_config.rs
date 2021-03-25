//! Program state processor

use crate::{
    state::timelock_config::TimelockConfig,
    state::timelock_program::TimelockProgram,
    utils::{assert_initialized, assert_token_program_is_correct, create_account_raw},
};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    pubkey::Pubkey,
};

/// Create empty timelock config
pub fn process_create_empty_timelock_config(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let timelock_config_account_info = next_account_info(account_info_iter)?;
    let program_to_tie_account_info = next_account_info(account_info_iter)?;
    let governance_mint_account_info = next_account_info(account_info_iter)?;
    let council_mint_account_info = next_account_info(account_info_iter)?;
    let payer_account_info = next_account_info(account_info_iter)?;
    let timelock_program_account_info = next_account_info(account_info_iter)?;
    let timelock_program_info = next_account_info(account_info_iter)?;
    let token_program_account_info = next_account_info(account_info_iter)?;
    let system_account_info = next_account_info(account_info_iter)?;

    let timelock_program: TimelockProgram = assert_initialized(timelock_program_account_info)?;
    assert_token_program_is_correct(&timelock_program, token_program_account_info)?;
    let seeds = &[
        timelock_program_account_info.key.as_ref(),
        governance_mint_account_info.key.as_ref(),
        council_mint_account_info.key.as_ref(),
        program_to_tie_account_info.key.as_ref(),
    ];
    let (config_key, bump_seed) = Pubkey::find_program_address(seeds, program_id);
    let authority_signer_seeds = &[
        timelock_program_account_info.key.as_ref(),
        governance_mint_account_info.key.as_ref(),
        council_mint_account_info.key.as_ref(),
        program_to_tie_account_info.key.as_ref(),
        &[bump_seed],
    ];

    create_account_raw::<TimelockConfig>(
        &[
            payer_account_info.clone(),
            timelock_program_info.clone(),
            timelock_config_account_info.clone(),
            system_account_info.clone(),
        ],
        &config_key,
        payer_account_info.key,
        program_id,
        authority_signer_seeds,
    )?;

    Ok(())
}
