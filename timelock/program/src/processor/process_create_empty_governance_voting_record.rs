//! Program state processor

use crate::{state::governance_voting_record::GovernanceVotingRecord, utils::create_account_raw};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    pubkey::Pubkey,
};

/// Create empty governance voting record
pub fn process_create_empty_governance_voting_record(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let voting_record_account_info = next_account_info(account_info_iter)?;
    let proposal_account_info = next_account_info(account_info_iter)?;
    let voting_account_info = next_account_info(account_info_iter)?;
    let payer_account_info = next_account_info(account_info_iter)?;
    let timelock_program_info = next_account_info(account_info_iter)?;
    let system_account_info = next_account_info(account_info_iter)?;

    let seeds = &[
        timelock_program_info.key.as_ref(),
        proposal_account_info.key.as_ref(),
        voting_account_info.key.as_ref(),
    ];
    let (voting_key, bump_seed) = Pubkey::find_program_address(seeds, program_id);
    let authority_signer_seeds = &[
        timelock_program_info.key.as_ref(),
        proposal_account_info.key.as_ref(),
        voting_account_info.key.as_ref(),
        &[bump_seed],
    ];

    create_account_raw::<GovernanceVotingRecord>(
        &[
            payer_account_info.clone(),
            timelock_program_info.clone(),
            voting_record_account_info.clone(),
            system_account_info.clone(),
        ],
        &voting_key,
        payer_account_info.key,
        program_id,
        authority_signer_seeds,
    )?;

    Ok(())
}
