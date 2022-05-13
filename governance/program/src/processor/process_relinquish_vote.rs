//! Program state processor

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    entrypoint::ProgramResult,
    pubkey::Pubkey,
    sysvar::Sysvar,
};
use spl_governance_tools::account::dispose_account;

use crate::{
    error::GovernanceError,
    state::{
        enums::ProposalState,
        governance::get_governance_data,
        proposal::get_proposal_data_for_governance_and_governing_mint,
        token_owner_record::get_token_owner_record_data_for_realm_and_governing_mint,
        vote_record::{get_vote_record_data_for_proposal_and_token_owner, Vote},
    },
};

/// Processes RelinquishVote instruction
pub fn process_relinquish_vote(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let governance_info = next_account_info(account_info_iter)?; // 0
    let proposal_info = next_account_info(account_info_iter)?; // 1
    let token_owner_record_info = next_account_info(account_info_iter)?; // 2

    let vote_record_info = next_account_info(account_info_iter)?; // 3
    let governing_token_mint_info = next_account_info(account_info_iter)?; // 4

    let governance_data = get_governance_data(program_id, governance_info)?;

    let mut proposal_data = get_proposal_data_for_governance_and_governing_mint(
        program_id,
        proposal_info,
        governance_info.key,
        governing_token_mint_info.key,
    )?;

    let mut token_owner_record_data = get_token_owner_record_data_for_realm_and_governing_mint(
        program_id,
        token_owner_record_info,
        &governance_data.realm,
        governing_token_mint_info.key,
    )?;

    let mut vote_record_data = get_vote_record_data_for_proposal_and_token_owner(
        program_id,
        vote_record_info,
        proposal_info.key,
        &token_owner_record_data.governing_token_owner,
    )?;
    vote_record_data.assert_can_relinquish_vote()?;

    let clock = Clock::get()?;

    // If the Proposal is still being voted on then the token owner vote will be withdrawn and it won't count towards the vote outcome
    // Note: If there is no tipping point the proposal can be still in Voting state but already past the configured max_voting_time
    //       It means it awaits manual finalization (FinalizeVote) and it should no longer be possible to withdraw the vote and we only release the tokens
    if proposal_data.state == ProposalState::Voting
        && !proposal_data.has_vote_time_ended(&governance_data.config, clock.unix_timestamp)
    {
        let governance_authority_info = next_account_info(account_info_iter)?; // 5
        let beneficiary_info = next_account_info(account_info_iter)?; // 6

        // Note: It's only required to sign by governing_authority if relinquishing the vote results in vote change
        // If the Proposal is already decided then anybody can prune active votes for token owner
        token_owner_record_data
            .assert_token_owner_or_delegate_is_signer(governance_authority_info)?;

        match vote_record_data.vote {
            Vote::Approve(choices) => {
                for (option, choice) in proposal_data.options.iter_mut().zip(choices) {
                    option.vote_weight = option
                        .vote_weight
                        .checked_sub(choice.get_choice_weight(vote_record_data.voter_weight)?)
                        .unwrap();
                }
            }
            Vote::Deny => {
                proposal_data.deny_vote_weight = Some(
                    proposal_data
                        .deny_vote_weight
                        .unwrap()
                        .checked_sub(vote_record_data.voter_weight)
                        .unwrap(),
                )
            }
            Vote::Abstain | Vote::Veto => {
                return Err(GovernanceError::NotSupportedVoteType.into());
            }
        }

        proposal_data.serialize(&mut *proposal_info.data.borrow_mut())?;

        dispose_account(vote_record_info, beneficiary_info);

        token_owner_record_data.total_votes_count = token_owner_record_data
            .total_votes_count
            .checked_sub(1)
            .unwrap();
    } else {
        vote_record_data.is_relinquished = true;
        vote_record_data.serialize(&mut *vote_record_info.data.borrow_mut())?;
    }

    // If the Proposal has been already voted on then we only have to decrease unrelinquished_votes_count
    token_owner_record_data.unrelinquished_votes_count = token_owner_record_data
        .unrelinquished_votes_count
        .checked_sub(1)
        .unwrap();

    token_owner_record_data.serialize(&mut *token_owner_record_info.data.borrow_mut())?;

    Ok(())
}
