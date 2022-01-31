//! Program state processor

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    entrypoint::ProgramResult,
    pubkey::Pubkey,
    sysvar::Sysvar,
};

use crate::state::{
    governance::get_governance_data_for_realm,
    proposal::get_proposal_data_for_governance_and_governing_mint,
    realm::get_realm_data_for_governing_token_mint,
    token_owner_record::get_token_owner_record_data_for_proposal_owner,
};

/// Processes FinalizeVote instruction
pub fn process_finalize_vote(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let realm_info = next_account_info(account_info_iter)?; // 0
    let governance_info = next_account_info(account_info_iter)?; // 1
    let proposal_info = next_account_info(account_info_iter)?; // 2
    let proposal_owner_record_info = next_account_info(account_info_iter)?; // 3

    let governing_token_mint_info = next_account_info(account_info_iter)?; // 4

    let clock = Clock::get()?;

    let mut realm_data = get_realm_data_for_governing_token_mint(
        program_id,
        realm_info,
        governing_token_mint_info.key,
    )?;
    let mut governance_data =
        get_governance_data_for_realm(program_id, governance_info, realm_info.key)?;

    let mut proposal_data = get_proposal_data_for_governance_and_governing_mint(
        program_id,
        proposal_info,
        governance_info.key,
        governing_token_mint_info.key,
    )?;

    let realm_config_info = next_account_info(account_info_iter)?; // 5

    let max_voter_weight = proposal_data.resolve_max_voter_weight(
        program_id,
        realm_config_info,
        governing_token_mint_info,
        account_info_iter, // *6
        realm_info.key,
        &realm_data,
    )?;

    proposal_data.finalize_vote(
        max_voter_weight,
        &governance_data.config,
        clock.unix_timestamp,
    )?;

    let mut proposal_owner_record_data = get_token_owner_record_data_for_proposal_owner(
        program_id,
        proposal_owner_record_info,
        &proposal_data.token_owner_record,
    )?;

    proposal_owner_record_data.decrease_outstanding_proposal_count();
    proposal_owner_record_data.serialize(&mut *proposal_owner_record_info.data.borrow_mut())?;

    proposal_data.serialize(&mut *proposal_info.data.borrow_mut())?;

    // Update Realm voting_proposal_count
    realm_data.voting_proposal_count = realm_data.voting_proposal_count.saturating_sub(1);
    realm_data.serialize(&mut *realm_info.data.borrow_mut())?;

    // Update  Governance voting_proposal_count
    governance_data.voting_proposal_count = governance_data.voting_proposal_count.saturating_sub(1);
    governance_data.serialize(&mut *governance_info.data.borrow_mut())?;

    Ok(())
}
