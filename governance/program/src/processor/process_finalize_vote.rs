//! Program state processor

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    entrypoint::ProgramResult,
    pubkey::Pubkey,
    sysvar::Sysvar,
};

use crate::{
    state::{
        governance::get_governance_data,
        proposal::get_proposal_data_for_governance_and_governing_mint,
    },
    tools::spl_token::get_spl_token_mint_supply,
};

use borsh::BorshSerialize;

/// Processes FinalizeVote instruction
pub fn process_finalize_vote(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let governance_info = next_account_info(account_info_iter)?; // 0
    let proposal_info = next_account_info(account_info_iter)?; // 1

    let governing_token_mint_info = next_account_info(account_info_iter)?; // 2

    let clock_info = next_account_info(account_info_iter)?; // 3
    let clock = Clock::from_account_info(clock_info)?;

    let governance_data = get_governance_data(program_id, governance_info)?;

    let mut proposal_data = get_proposal_data_for_governance_and_governing_mint(
        program_id,
        &proposal_info,
        governance_info.key,
        governing_token_mint_info.key,
    )?;

    let governing_token_supply = get_spl_token_mint_supply(&governing_token_mint_info)?;

    proposal_data.finalize_vote(governing_token_supply, &governance_data.config, clock.slot)?;

    proposal_data.serialize(&mut *proposal_info.data.borrow_mut())?;

    Ok(())
}
