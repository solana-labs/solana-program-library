//! Program state processor

use crate::{
    error::GovernanceError, state::governance::Governance, utils::create_account_raw,
    PROGRAM_AUTHORITY_SEED,
};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    pubkey::Pubkey,
};

/// Create empty Governance
pub fn process_create_empty_governance(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let governance_account_info = next_account_info(account_info_iter)?; // 0
    let program_to_tie_account_info = next_account_info(account_info_iter)?; // 1
    let program_upgrade_authority_account_info = next_account_info(account_info_iter)?; //2
    let governance_mint_account_info = next_account_info(account_info_iter)?; // 3
    let payer_account_info = next_account_info(account_info_iter)?; //4
    let system_account_info = next_account_info(account_info_iter)?; //5

    let council_mint_seed = next_account_info(account_info_iter) //6
        .map(|acc| acc.key.as_ref())
        .unwrap_or(&[]);

    let mut seeds = vec![
        PROGRAM_AUTHORITY_SEED,
        program_id.as_ref(),
        governance_mint_account_info.key.as_ref(),
        council_mint_seed,
        program_to_tie_account_info.key.as_ref(),
    ];
    let (governance_key, bump_seed) = Pubkey::find_program_address(&seeds[..], program_id);

    if governance_account_info.key != &governance_key {
        return Err(GovernanceError::InvalidGovernanceKey.into());
    }
    let bump = &[bump_seed];
    seeds.push(bump);
    let authority_signer_seeds = &seeds[..];

    create_account_raw::<Governance>(
        &[
            payer_account_info.clone(),
            governance_account_info.clone(),
            system_account_info.clone(),
        ],
        &governance_key,
        payer_account_info.key,
        program_id,
        authority_signer_seeds,
    )?;

    Ok(())
}
