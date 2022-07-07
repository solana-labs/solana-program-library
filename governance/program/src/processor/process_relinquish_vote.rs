//! Program state processor

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    entrypoint::ProgramResult,
    pubkey::Pubkey,
    sysvar::Sysvar,
};
use spl_governance_addin_api::voter_weight::VoterWeightAction;
use spl_governance_tools::account::dispose_account;

use crate::{
    addins::voter_weight::{
        assert_is_valid_voter_weight,
        get_revoke_delegated_voter_weight_record_data_for_token_owner_record,
    },
    error::GovernanceError,
    state::{
        enums::ProposalState,
        governance::get_governance_data_for_realm,
        proposal::get_proposal_data_for_governance,
        realm::get_realm_data_for_governing_token_mint,
        realm_config::get_realm_config_data_for_realm,
        token_owner_record::get_token_owner_record_data_for_realm_and_governing_mint,
        vote_record::{get_vote_record_data_for_proposal_and_token_owner_record, Vote},
    },
};

/// Processes RelinquishVote instruction
pub fn process_relinquish_vote(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let realm_info = next_account_info(account_info_iter)?; // 0
    let governance_info = next_account_info(account_info_iter)?; // 1
    let proposal_info = next_account_info(account_info_iter)?; // 2
    let token_owner_record_info = next_account_info(account_info_iter)?; // 3

    let vote_record_info = next_account_info(account_info_iter)?; // 4
    let vote_governing_token_mint_info = next_account_info(account_info_iter)?; // 5

    let realm_data = get_realm_data_for_governing_token_mint(
        program_id,
        realm_info,
        vote_governing_token_mint_info.key,
    )?;

    let governance_data =
        get_governance_data_for_realm(program_id, governance_info, realm_info.key)?;

    let mut proposal_data =
        get_proposal_data_for_governance(program_id, proposal_info, governance_info.key)?;

    let mut token_owner_record_data = get_token_owner_record_data_for_realm_and_governing_mint(
        program_id,
        token_owner_record_info,
        &governance_data.realm,
        vote_governing_token_mint_info.key,
    )?;

    let mut vote_record_data = get_vote_record_data_for_proposal_and_token_owner_record(
        program_id,
        vote_record_info,
        &realm_data,
        proposal_info.key,
        &proposal_data,
        &token_owner_record_data,
    )?;
    vote_record_data.assert_can_relinquish_vote()?;

    let clock = Clock::get()?;

    // If the Proposal is still being voted on then the token owner vote will be withdrawn and it won't count towards the vote outcome
    // Note: If there is no tipping point the proposal can be still in Voting state but already past the configured max_voting_time
    //       It means it awaits manual finalization (FinalizeVote) and it should no longer be possible to withdraw the vote
    if proposal_data.state == ProposalState::Voting
        && !proposal_data.has_vote_time_ended(&governance_data.config, clock.unix_timestamp)
    {
        let governance_authority_info = next_account_info(account_info_iter)?; // 6
        let beneficiary_info = next_account_info(account_info_iter)?; // 7

        let max_weight_to_relinquish = match next_account_info(account_info_iter) {
            // 8
            Ok(relinquish_info) => {
                let realm_config_data = get_realm_config_data_for_realm(
                    program_id,
                    next_account_info(account_info_iter)?, // 9
                    realm_info.key,
                )?;
                let relinquish_data =
                    get_revoke_delegated_voter_weight_record_data_for_token_owner_record(
                        &realm_config_data
                            .community_token_config
                            .voter_weight_addin
                            .unwrap(),
                        relinquish_info,
                        &token_owner_record_data,
                        governance_authority_info,
                    )?;

                assert_is_valid_voter_weight(
                    &relinquish_data,
                    VoterWeightAction::RevokeVote,
                    vote_record_info.key,
                )?;

                relinquish_data.voter_weight
            }
            Err(_) => {
                // Note: It's only required to sign by governing_authority if relinquishing the vote results in vote change
                // If the Proposal is already decided then anybody can prune active votes for token owner
                token_owner_record_data
                    .assert_token_owner_or_delegate_is_signer(governance_authority_info)?;
                u64::max_value()
            }
        };

        let weight_to_relinquish = vote_record_data.voter_weight.min(max_weight_to_relinquish);

        match vote_record_data.vote {
            Vote::Approve(ref choices) => {
                for (option, choice) in proposal_data.options.iter_mut().zip(choices) {
                    option.vote_weight = option
                        .vote_weight
                        .checked_sub(choice.get_choice_weight(weight_to_relinquish)?)
                        .unwrap();
                }
            }
            Vote::Deny => {
                proposal_data.deny_vote_weight = Some(
                    proposal_data
                        .deny_vote_weight
                        .unwrap()
                        .checked_sub(weight_to_relinquish)
                        .unwrap(),
                )
            }
            Vote::Veto => {
                proposal_data.veto_vote_weight = proposal_data
                    .veto_vote_weight
                    .checked_sub(weight_to_relinquish)
                    .unwrap();
            }
            Vote::Abstain => {
                return Err(GovernanceError::NotSupportedVoteType.into());
            }
        }

        proposal_data.serialize(&mut *proposal_info.data.borrow_mut())?;

        if vote_record_data.voter_weight <= weight_to_relinquish {
            dispose_account(vote_record_info, beneficiary_info);

            token_owner_record_data.total_votes_count = token_owner_record_data
                .total_votes_count
                .checked_sub(1)
                .unwrap();
        } else {
            vote_record_data.voter_weight = vote_record_data
                .voter_weight
                .checked_sub(weight_to_relinquish)
                .unwrap();
            vote_record_data.serialize(&mut *vote_record_info.data.borrow_mut())?;

            return Ok(());
        }
    } else {
        // After Proposal voting time ends and it's not tipped then it enters implicit (time based) Finalizing state
        // and releasing tokens in this state should be disallowed
        // In other words releasing tokens is only possible once Proposal is manually finalized using FinalizeVote
        if proposal_data.state == ProposalState::Voting {
            return Err(GovernanceError::CannotRelinquishInFinalizingState.into());
        }

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
