//! Program state processor

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    entrypoint::ProgramResult,
    pubkey::Pubkey,
    rent::Rent,
    sysvar::Sysvar,
};
use spl_governance_tools::account::create_and_serialize_account_signed;

use crate::{
    error::GovernanceError,
    state::{
        enums::GovernanceAccountType,
        governance::get_governance_data_for_realm,
        proposal::get_proposal_data_for_governance_and_governing_mint,
        realm::get_realm_data_for_governing_token_mint,
        token_owner_record::{
            get_token_owner_record_data_for_proposal_owner,
            get_token_owner_record_data_for_realm_and_governing_mint,
        },
        vote_record::{get_vote_record_address_seeds, Vote, VoteRecordV2},
    },
    tools::spl_token::get_spl_token_mint_supply,
};

use borsh::BorshSerialize;

/// Processes CastVote instruction
pub fn process_cast_vote(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    vote: Vote,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let realm_info = next_account_info(account_info_iter)?; // 0
    let governance_info = next_account_info(account_info_iter)?; // 1

    let proposal_info = next_account_info(account_info_iter)?; // 2
    let proposal_owner_record_info = next_account_info(account_info_iter)?; // 3

    let voter_token_owner_record_info = next_account_info(account_info_iter)?; // 4
    let governance_authority_info = next_account_info(account_info_iter)?; // 5

    let vote_record_info = next_account_info(account_info_iter)?; // 6
    let governing_token_mint_info = next_account_info(account_info_iter)?; // 7

    let payer_info = next_account_info(account_info_iter)?; // 8
    let system_info = next_account_info(account_info_iter)?; // 9

    let rent_sysvar_info = next_account_info(account_info_iter)?; // 10
    let rent = &Rent::from_account_info(rent_sysvar_info)?;

    let clock_info = next_account_info(account_info_iter)?; // 11
    let clock = Clock::from_account_info(clock_info)?;

    if !vote_record_info.data_is_empty() {
        return Err(GovernanceError::VoteAlreadyExists.into());
    }

    let realm_data = get_realm_data_for_governing_token_mint(
        program_id,
        realm_info,
        governing_token_mint_info.key,
    )?;
    let governance_data =
        get_governance_data_for_realm(program_id, governance_info, realm_info.key)?;

    let mut proposal_data = get_proposal_data_for_governance_and_governing_mint(
        program_id,
        proposal_info,
        governance_info.key,
        governing_token_mint_info.key,
    )?;
    proposal_data.assert_can_cast_vote(&governance_data.config, clock.unix_timestamp)?;

    let mut voter_token_owner_record_data =
        get_token_owner_record_data_for_realm_and_governing_mint(
            program_id,
            voter_token_owner_record_info,
            &governance_data.realm,
            governing_token_mint_info.key,
        )?;
    voter_token_owner_record_data
        .assert_token_owner_or_delegate_is_signer(governance_authority_info)?;

    // Update TokenOwnerRecord vote counts
    voter_token_owner_record_data.unrelinquished_votes_count = voter_token_owner_record_data
        .unrelinquished_votes_count
        .checked_add(1)
        .unwrap();

    voter_token_owner_record_data.total_votes_count = voter_token_owner_record_data
        .total_votes_count
        .checked_add(1)
        .unwrap();

    let voter_weight = voter_token_owner_record_data.resolve_voter_weight(
        program_id,
        account_info_iter,
        realm_info.key,
        &realm_data,
    )?;

    proposal_data.assert_valid_vote(&vote)?;

    // Calculate Proposal voting weights
    match &vote {
        Vote::Approve(choices) => {
            for (option, choice) in proposal_data.options.iter_mut().zip(choices) {
                option.vote_weight = option
                    .vote_weight
                    .checked_add(choice.get_choice_weight(voter_weight)?)
                    .unwrap();
            }
        }
        Vote::Deny => {
            proposal_data.deny_vote_weight = Some(
                proposal_data
                    .deny_vote_weight
                    .unwrap()
                    .checked_add(voter_weight)
                    .unwrap(),
            )
        }
    }

    let governing_token_mint_supply = get_spl_token_mint_supply(governing_token_mint_info)?;
    if proposal_data.try_tip_vote(
        governing_token_mint_supply,
        &governance_data.config,
        &realm_data,
        clock.unix_timestamp,
    )? {
        if proposal_owner_record_info.key == voter_token_owner_record_info.key {
            voter_token_owner_record_data.decrease_outstanding_proposal_count();
        } else {
            let mut proposal_owner_record_data = get_token_owner_record_data_for_proposal_owner(
                program_id,
                proposal_owner_record_info,
                &proposal_data.token_owner_record,
            )?;
            proposal_owner_record_data.decrease_outstanding_proposal_count();
            proposal_owner_record_data
                .serialize(&mut *proposal_owner_record_info.data.borrow_mut())?;
        };
    }

    voter_token_owner_record_data
        .serialize(&mut *voter_token_owner_record_info.data.borrow_mut())?;

    proposal_data.serialize(&mut *proposal_info.data.borrow_mut())?;

    // Create and serialize VoteRecord
    let vote_record_data = VoteRecordV2 {
        account_type: GovernanceAccountType::VoteRecordV2,
        proposal: *proposal_info.key,
        governing_token_owner: voter_token_owner_record_data.governing_token_owner,
        voter_weight,
        vote,
        is_relinquished: false,
    };

    create_and_serialize_account_signed::<VoteRecordV2>(
        payer_info,
        vote_record_info,
        &vote_record_data,
        &get_vote_record_address_seeds(proposal_info.key, voter_token_owner_record_info.key),
        program_id,
        system_info,
        rent,
    )?;

    Ok(())
}
