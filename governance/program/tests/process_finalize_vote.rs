#![cfg(feature = "test-bpf")]

mod program_test;

use solana_program_test::tokio;

use program_test::*;
use spl_governance::{
    error::GovernanceError,
    state::enums::{ProposalState, VoteThresholdPercentage},
};

#[tokio::test]
async fn test_finalize_vote_to_succeeded() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    let mut governance_config = governance_test.get_default_governance_config();

    governance_config.vote_threshold_percentage = VoteThresholdPercentage::YesVote(40);

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let mut account_governance_cookie = governance_test
        .with_account_governance_using_config(
            &realm_cookie,
            &governed_account_cookie,
            &token_owner_record_cookie,
            &governance_config,
        )
        .await
        .unwrap();

    // Total 210 tokens
    governance_test
        .mint_community_tokens(&realm_cookie, 110)
        .await;

    let proposal_cookie = governance_test
        .with_signed_off_proposal(&token_owner_record_cookie, &mut account_governance_cookie)
        .await
        .unwrap();

    governance_test
        .with_cast_vote(&proposal_cookie, &token_owner_record_cookie, YesNoVote::Yes)
        .await
        .unwrap();

    // Ensure not tipped
    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    assert_eq!(ProposalState::Voting, proposal_account.state);

    // Advance timestamp past max_voting_time
    governance_test
        .advance_clock_past_timestamp(
            account_governance_cookie.account.config.max_voting_time as i64
                + proposal_account.voting_at.unwrap(),
        )
        .await;

    let clock = governance_test.bench.get_clock().await;

    // Act

    governance_test
        .finalize_vote(&realm_cookie, &proposal_cookie)
        .await
        .unwrap();

    // Assert

    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    assert_eq!(proposal_account.state, ProposalState::Succeeded);
    assert_eq!(
        Some(clock.unix_timestamp),
        proposal_account.voting_completed_at
    );

    assert_eq!(Some(210), proposal_account.max_vote_weight);

    assert_eq!(
        Some(
            account_governance_cookie
                .account
                .config
                .vote_threshold_percentage
        ),
        proposal_account.vote_threshold_percentage
    );

    let proposal_owner_record = governance_test
        .get_token_owner_record_account(&proposal_cookie.account.token_owner_record)
        .await;

    assert_eq!(0, proposal_owner_record.outstanding_proposal_count);
}

#[tokio::test]
async fn test_finalize_vote_to_defeated() {
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

    governance_test
        .with_cast_vote(&proposal_cookie, &token_owner_record_cookie, YesNoVote::No)
        .await
        .unwrap();

    // Ensure not tipped
    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    assert_eq!(ProposalState::Voting, proposal_account.state);

    // Advance clock past max_voting_time
    governance_test
        .advance_clock_past_timestamp(
            account_governance_cookie.account.config.max_voting_time as i64
                + proposal_account.voting_at.unwrap(),
        )
        .await;

    // Act

    governance_test
        .finalize_vote(&realm_cookie, &proposal_cookie)
        .await
        .unwrap();

    // Assert

    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    assert_eq!(ProposalState::Defeated, proposal_account.state);
}

#[tokio::test]
async fn test_finalize_vote_with_invalid_mint_error() {
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

    let mut proposal_cookie = governance_test
        .with_signed_off_proposal(&token_owner_record_cookie, &mut account_governance_cookie)
        .await
        .unwrap();

    governance_test
        .with_cast_vote(&proposal_cookie, &token_owner_record_cookie, YesNoVote::No)
        .await
        .unwrap();

    // Ensure not tipped
    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    assert_eq!(ProposalState::Voting, proposal_account.state);

    proposal_cookie.account.governing_token_mint =
        realm_cookie.account.config.council_mint.unwrap();

    // Act

    let err = governance_test
        .finalize_vote(&realm_cookie, &proposal_cookie)
        .await
        .err()
        .unwrap();

    // Assert

    assert_eq!(err, GovernanceError::InvalidGoverningMintForProposal.into());
}

#[tokio::test]
async fn test_finalize_vote_with_invalid_governance_error() {
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

    let mut proposal_cookie = governance_test
        .with_signed_off_proposal(&token_owner_record_cookie, &mut account_governance_cookie)
        .await
        .unwrap();

    governance_test
        .with_cast_vote(&proposal_cookie, &token_owner_record_cookie, YesNoVote::No)
        .await
        .unwrap();

    // Ensure not tipped
    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    assert_eq!(ProposalState::Voting, proposal_account.state);

    // Setup Governance for a different account
    let governed_account_cookie2 = governance_test.with_governed_account().await;

    let account_governance_cookie2 = governance_test
        .with_account_governance(
            &realm_cookie,
            &governed_account_cookie2,
            &token_owner_record_cookie,
        )
        .await
        .unwrap();

    proposal_cookie.account.governance = account_governance_cookie2.address;

    // Act

    let err = governance_test
        .finalize_vote(&realm_cookie, &proposal_cookie)
        .await
        .err()
        .unwrap();

    // Assert

    assert_eq!(err, GovernanceError::InvalidGovernanceForProposal.into());
}
