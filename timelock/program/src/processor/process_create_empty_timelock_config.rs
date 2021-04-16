//! Program state processor

use crate::{state::timelock_config::TimelockConfig, utils::create_account_raw};
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
    let payer_account_info = next_account_info(account_info_iter)?;
    let timelock_program_account_info = next_account_info(account_info_iter)?;
    let system_account_info = next_account_info(account_info_iter)?;

    let council_mint_seed = next_account_info(account_info_iter)
        .map(|acc| acc.key.as_ref())
        .unwrap_or(&[]);

    let seeds = &[
        timelock_program_account_info.key.as_ref(),
        governance_mint_account_info.key.as_ref(),
        council_mint_seed,
        program_to_tie_account_info.key.as_ref(),
    ];
    let (config_key, bump_seed) = Pubkey::find_program_address(seeds, program_id);

    let authority_signer_seeds = &[
        timelock_program_account_info.key.as_ref(),
        governance_mint_account_info.key.as_ref(),
        council_mint_seed,
        program_to_tie_account_info.key.as_ref(),
        &[bump_seed],
    ];

    create_account_raw::<TimelockConfig>(
        &[
            payer_account_info.clone(),
            timelock_program_account_info.clone(),
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
