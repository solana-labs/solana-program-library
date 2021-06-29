//! Program state processor

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    entrypoint::ProgramResult,
    pubkey::Pubkey,
    rent::Rent,
    sysvar::Sysvar,
};

use crate::{
    error::GovernanceError,
    instruction::Vote,
    state::{
        enums::{GovernanceAccountType, VoteWeight},
        governance::get_governance_data,
        proposal::get_proposal_data_for_governance_and_governing_mint,
        token_owner_record::get_token_owner_record_data_for_realm_and_governing_mint,
        vote_record::{get_vote_record_address_seeds, VoteRecord},
    },
    tools::{account::create_and_serialize_account_signed, spl_token::get_spl_token_mint_supply},
};

use borsh::BorshSerialize;

/// Processes CastVote instruction
pub fn process_cast_vote(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    vote: Vote,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let governance_info = next_account_info(account_info_iter)?; // 0
    let proposal_info = next_account_info(account_info_iter)?; // 1
    let token_owner_record_info = next_account_info(account_info_iter)?; // 2
    let governance_authority_info = next_account_info(account_info_iter)?; // 3

    let vote_record_info = next_account_info(account_info_iter)?; // 4
    let governing_token_mint_info = next_account_info(account_info_iter)?; // 5

    let payer_info = next_account_info(account_info_iter)?; // 6
    let system_info = next_account_info(account_info_iter)?; // 7

    let rent_sysvar_info = next_account_info(account_info_iter)?; // 8
    let rent = &Rent::from_account_info(rent_sysvar_info)?;

    let clock_info = next_account_info(account_info_iter)?; // 9
    let clock = Clock::from_account_info(clock_info)?;

    if !vote_record_info.data_is_empty() {
        return Err(GovernanceError::VoteAlreadyExists.into());
    }

    let governance_data = get_governance_data(program_id, governance_info)?;

    let mut proposal_data = get_proposal_data_for_governance_and_governing_mint(
        program_id,
        proposal_info,
        governance_info.key,
        governing_token_mint_info.key,
    )?;
    proposal_data.assert_can_cast_vote(&governance_data.config, clock.unix_timestamp)?;

    let mut token_owner_record_data = get_token_owner_record_data_for_realm_and_governing_mint(
        program_id,
        token_owner_record_info,
        &governance_data.config.realm,
        governing_token_mint_info.key,
    )?;
    token_owner_record_data.assert_token_owner_or_delegate_is_signer(governance_authority_info)?;

    // Update TokenOwnerRecord vote counts
    token_owner_record_data.unrelinquished_votes_count = token_owner_record_data
        .unrelinquished_votes_count
        .checked_add(1)
        .unwrap();

    token_owner_record_data.total_votes_count = token_owner_record_data
        .total_votes_count
        .checked_add(1)
        .unwrap();

    token_owner_record_data.serialize(&mut *token_owner_record_info.data.borrow_mut())?;

    let vote_amount = token_owner_record_data.governing_token_deposit_amount;

    // Calculate Proposal voting weights
    let vote_weight = match vote {
        Vote::Yes => {
            proposal_data.yes_votes_count = proposal_data
                .yes_votes_count
                .checked_add(vote_amount)
                .unwrap();
            VoteWeight::Yes(vote_amount)
        }
        Vote::No => {
            proposal_data.no_votes_count = proposal_data
                .no_votes_count
                .checked_add(vote_amount)
                .unwrap();
            VoteWeight::No(vote_amount)
        }
    };

    let governing_token_supply = get_spl_token_mint_supply(governing_token_mint_info)?;
    proposal_data.try_tip_vote(
        governing_token_supply,
        &governance_data.config,
        clock.unix_timestamp,
    );

    proposal_data.serialize(&mut *proposal_info.data.borrow_mut())?;

    // Create and serialize VoteRecord
    let vote_record_data = VoteRecord {
        account_type: GovernanceAccountType::VoteRecord,
        proposal: *proposal_info.key,
        governing_token_owner: token_owner_record_data.governing_token_owner,
        vote_weight,
        is_relinquished: false,
    };

    create_and_serialize_account_signed::<VoteRecord>(
        payer_info,
        vote_record_info,
        &vote_record_data,
        &get_vote_record_address_seeds(proposal_info.key, token_owner_record_info.key),
        program_id,
        system_info,
        rent,
    )?;

    Ok(())
}
