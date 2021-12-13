#![cfg(feature = "test-bpf")]

mod program_test;

use solana_program::{instruction::AccountMeta, pubkey::Pubkey};
use solana_program_test::tokio;

use program_test::*;
use spl_governance::{error::GovernanceError, state::enums::ProposalState};

#[tokio::test]
async fn test_relinquish_voted_proposal() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let mut account_governance_cookie = governance_test
        .with_account_governance(
            &realm_cookie,
            &governed_account_cookie,
            &token_owner_record_cookie,
        )
        .await
        .unwrap();

    let proposal_cookie = governance_test
        .with_signed_off_proposal(&token_owner_record_cookie, &mut account_governance_cookie)
        .await
        .unwrap();

    let mut vote_record_cookie = governance_test
        .with_cast_vote(&proposal_cookie, &token_owner_record_cookie, YesNoVote::Yes)
        .await
        .unwrap();

    // Act
    governance_test
        .relinquish_vote(&proposal_cookie, &token_owner_record_cookie)
        .await
        .unwrap();

    // Assert

    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    assert_eq!(100, proposal_account.options[0].vote_weight);
    assert_eq!(ProposalState::Succeeded, proposal_account.state);

    let token_owner_record = governance_test
        .get_token_owner_record_account(&token_owner_record_cookie.address)
        .await;

    assert_eq!(0, token_owner_record.unrelinquished_votes_count);
    assert_eq!(1, token_owner_record.total_votes_count);

    let vote_record_account = governance_test
        .get_vote_record_account(&vote_record_cookie.address)
        .await;

    vote_record_cookie.account.is_relinquished = true;
    assert_eq!(vote_record_cookie.account, vote_record_account);
}

#[tokio::test]
async fn test_relinquish_active_yes_vote() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let mut account_governance_cookie = governance_test
        .with_account_governance(
            &realm_cookie,
            &governed_account_cookie,
            &token_owner_record_cookie,
        )
        .await
        .unwrap();

    // Total 300 tokens
    governance_test
        .mint_community_tokens(&realm_cookie, 200)
        .await;

    let proposal_cookie = governance_test
        .with_signed_off_proposal(&token_owner_record_cookie, &mut account_governance_cookie)
        .await
        .unwrap();

    let vote_record_cookie = governance_test
        .with_cast_vote(&proposal_cookie, &token_owner_record_cookie, YesNoVote::Yes)
        .await
        .unwrap();

    // Act
    governance_test
        .relinquish_vote(&proposal_cookie, &token_owner_record_cookie)
        .await
        .unwrap();

    // Assert

    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    assert_eq!(0, proposal_account.options[0].vote_weight);
    assert_eq!(0, proposal_account.deny_vote_weight.unwrap());
    assert_eq!(ProposalState::Voting, proposal_account.state);

    let token_owner_record = governance_test
        .get_token_owner_record_account(&token_owner_record_cookie.address)
        .await;

    assert_eq!(0, token_owner_record.unrelinquished_votes_count);
    assert_eq!(0, token_owner_record.total_votes_count);

    let vote_record_account = governance_test
        .bench
        .get_account(&vote_record_cookie.address)
        .await;

    assert_eq!(None, vote_record_account);
}

#[tokio::test]
async fn test_relinquish_active_no_vote() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let mut account_governance_cookie = governance_test
        .with_account_governance(
            &realm_cookie,
            &governed_account_cookie,
            &token_owner_record_cookie,
        )
        .await
        .unwrap();

    // Total 300 tokens
    governance_test
        .mint_community_tokens(&realm_cookie, 200)
        .await;

    let proposal_cookie = governance_test
        .with_signed_off_proposal(&token_owner_record_cookie, &mut account_governance_cookie)
        .await
        .unwrap();

    let vote_record_cookie = governance_test
        .with_cast_vote(&proposal_cookie, &token_owner_record_cookie, YesNoVote::No)
        .await
        .unwrap();

    // Act
    governance_test
        .relinquish_vote(&proposal_cookie, &token_owner_record_cookie)
        .await
        .unwrap();

    // Assert

    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    assert_eq!(0, proposal_account.options[0].vote_weight);
    assert_eq!(0, proposal_account.deny_vote_weight.unwrap());
    assert_eq!(ProposalState::Voting, proposal_account.state);

    let token_owner_record = governance_test
        .get_token_owner_record_account(&token_owner_record_cookie.address)
        .await;

    assert_eq!(0, token_owner_record.unrelinquished_votes_count);
    assert_eq!(0, token_owner_record.total_votes_count);

    let vote_record_account = governance_test
        .bench
        .get_account(&vote_record_cookie.address)
        .await;

    assert_eq!(None, vote_record_account);
}

#[tokio::test]
async fn test_relinquish_vote_with_invalid_mint_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let mut account_governance_cookie = governance_test
        .with_account_governance(
            &realm_cookie,
            &governed_account_cookie,
            &token_owner_record_cookie,
        )
        .await
        .unwrap();

    let mut proposal_cookie = governance_test
        .with_signed_off_proposal(&token_owner_record_cookie, &mut account_governance_cookie)
        .await
        .unwrap();

    governance_test
        .with_cast_vote(&proposal_cookie, &token_owner_record_cookie, YesNoVote::No)
        .await
        .unwrap();

    proposal_cookie.account.governing_token_mint = Pubkey::new_unique();

    // Act

    let err = governance_test
        .relinquish_vote(&proposal_cookie, &token_owner_record_cookie)
        .await
        .err()
        .unwrap();

    // Assert

    assert_eq!(err, GovernanceError::InvalidGoverningMintForProposal.into());
}

#[tokio::test]
async fn test_relinquish_vote_with_governance_authority_must_sign_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    let mut token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let mut account_governance_cookie = governance_test
        .with_account_governance(
            &realm_cookie,
            &governed_account_cookie,
            &token_owner_record_cookie,
        )
        .await
        .unwrap();

    // Total 300 tokens
    governance_test
        .mint_community_tokens(&realm_cookie, 200)
        .await;

    let proposal_cookie = governance_test
        .with_signed_off_proposal(&token_owner_record_cookie, &mut account_governance_cookie)
        .await
        .unwrap();

    governance_test
        .with_cast_vote(&proposal_cookie, &token_owner_record_cookie, YesNoVote::No)
        .await
        .unwrap();

    // Try to use a different owner to sign
    let token_owner_record_cookie2 = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    token_owner_record_cookie.token_owner = token_owner_record_cookie2.token_owner;

    // Act

    let err = governance_test
        .relinquish_vote(&proposal_cookie, &token_owner_record_cookie)
        .await
        .err()
        .unwrap();

    // Assert

    assert_eq!(
        err,
        GovernanceError::GoverningTokenOwnerOrDelegateMustSign.into()
    );
}

#[tokio::test]
async fn test_relinquish_vote_with_invalid_vote_record_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let mut account_governance_cookie = governance_test
        .with_account_governance(
            &realm_cookie,
            &governed_account_cookie,
            &token_owner_record_cookie,
        )
        .await
        .unwrap();

    let token_owner_record_cookie2 = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    // Total 400 tokens
    governance_test
        .mint_community_tokens(&realm_cookie, 200)
        .await;

    let proposal_cookie = governance_test
        .with_signed_off_proposal(&token_owner_record_cookie, &mut account_governance_cookie)
        .await
        .unwrap();

    governance_test
        .with_cast_vote(&proposal_cookie, &token_owner_record_cookie, YesNoVote::No)
        .await
        .unwrap();

    let vote_record_cookie2 = governance_test
        .with_cast_vote(
            &proposal_cookie,
            &token_owner_record_cookie2,
            YesNoVote::Yes,
        )
        .await
        .unwrap();

    // // Act

    let err = governance_test
        .relinquish_vote_using_instruction(&proposal_cookie, &token_owner_record_cookie, |i| {
            i.accounts[3] = AccountMeta::new(vote_record_cookie2.address, false)
            // Try to use a vote_record for other token owner
        })
        .await
        .err()
        .unwrap();

    // // Assert

    assert_eq!(
        err,
        GovernanceError::InvalidGoverningTokenOwnerForVoteRecord.into()
    );
}

#[tokio::test]
async fn test_relinquish_vote_with_already_relinquished_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let mut account_governance_cookie = governance_test
        .with_account_governance(
            &realm_cookie,
            &governed_account_cookie,
            &token_owner_record_cookie,
        )
        .await
        .unwrap();

    let proposal_cookie = governance_test
        .with_signed_off_proposal(&token_owner_record_cookie, &mut account_governance_cookie)
        .await
        .unwrap();

    let vote_record_cookie = governance_test
        .with_cast_vote(&proposal_cookie, &token_owner_record_cookie, YesNoVote::No)
        .await
        .unwrap();

    governance_test
        .relinquish_vote(&proposal_cookie, &token_owner_record_cookie)
        .await
        .unwrap();

    // Ensure vote is relinquished
    let vote_record_account = governance_test
        .get_vote_record_account(&vote_record_cookie.address)
        .await;

    assert_eq!(true, vote_record_account.is_relinquished);

    governance_test
        .mint_community_tokens(&realm_cookie, 10)
        .await;

    governance_test.advance_clock().await;
    // Act

    let err = governance_test
        .relinquish_vote(&proposal_cookie, &token_owner_record_cookie)
        .await
        .err()
        .unwrap();

    // Assert

    assert_eq!(err, GovernanceError::VoteAlreadyRelinquished.into());
}

#[tokio::test]
async fn test_relinquish_proposal_in_voting_state_after_vote_time_ended() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    // Deposit 100 tokens
    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    // Add 200 tokens (total 300) to prevent the vote being tipped
    governance_test
        .mint_community_tokens(&realm_cookie, 200)
        .await;

    let mut account_governance_cookie = governance_test
        .with_account_governance(
            &realm_cookie,
            &governed_account_cookie,
            &token_owner_record_cookie,
        )
        .await
        .unwrap();

    let proposal_cookie = governance_test
        .with_signed_off_proposal(&token_owner_record_cookie, &mut account_governance_cookie)
        .await
        .unwrap();

    let clock = governance_test.bench.get_clock().await;

    let mut vote_record_cookie = governance_test
        .with_cast_vote(&proposal_cookie, &token_owner_record_cookie, YesNoVote::Yes)
        .await
        .unwrap();

    // Advance timestamp past max_voting_time
    governance_test
        .advance_clock_past_timestamp(
            account_governance_cookie.account.config.max_voting_time as i64 + clock.unix_timestamp,
        )
        .await;

    // Act
    governance_test
        .relinquish_vote(&proposal_cookie, &token_owner_record_cookie)
        .await
        .unwrap();

    // Assert

    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    // Proposal should be still in voting state but the vote count should not change
    assert_eq!(100, proposal_account.options[0].vote_weight);
    assert_eq!(ProposalState::Voting, proposal_account.state);

    let token_owner_record = governance_test
        .get_token_owner_record_account(&token_owner_record_cookie.address)
        .await;

    assert_eq!(0, token_owner_record.unrelinquished_votes_count);
    assert_eq!(1, token_owner_record.total_votes_count);

    let vote_record_account = governance_test
        .get_vote_record_account(&vote_record_cookie.address)
        .await;

    vote_record_cookie.account.is_relinquished = true;
    assert_eq!(vote_record_cookie.account, vote_record_account);
}
