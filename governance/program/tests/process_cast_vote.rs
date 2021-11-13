#![cfg(feature = "test-bpf")]

mod program_test;

use solana_program_test::tokio;

use program_test::*;
use spl_governance::{
    error::GovernanceError,
    state::enums::{ProposalState, VoteThresholdPercentage},
};

#[tokio::test]
async fn test_cast_vote() {
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

    let clock = governance_test.bench.get_clock().await;

    // Act
    let vote_record_cookie = governance_test
        .with_cast_vote(&proposal_cookie, &token_owner_record_cookie, YesNoVote::Yes)
        .await
        .unwrap();

    // Assert
    let vote_record_account = governance_test
        .get_vote_record_account(&vote_record_cookie.address)
        .await;

    assert_eq!(vote_record_cookie.account, vote_record_account);

    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    assert_eq!(
        token_owner_record_cookie
            .account
            .governing_token_deposit_amount,
        proposal_account.options[0].vote_weight
    );

    assert_eq!(proposal_account.state, ProposalState::Succeeded);
    assert_eq!(
        proposal_account.voting_completed_at,
        Some(clock.unix_timestamp)
    );

    assert_eq!(Some(100), proposal_account.max_vote_weight);
    assert_eq!(
        Some(
            account_governance_cookie
                .account
                .config
                .vote_threshold_percentage
        ),
        proposal_account.vote_threshold_percentage
    );

    let token_owner_record = governance_test
        .get_token_owner_record_account(&token_owner_record_cookie.address)
        .await;

    assert_eq!(1, token_owner_record.unrelinquished_votes_count);
    assert_eq!(1, token_owner_record.total_votes_count);
}

#[tokio::test]
async fn test_cast_vote_with_invalid_governance_error() {
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
        .with_cast_vote(&proposal_cookie, &token_owner_record_cookie, YesNoVote::Yes)
        .await
        .err()
        .unwrap();

    assert_eq!(err, GovernanceError::InvalidGovernanceForProposal.into());
}

#[tokio::test]
async fn test_cast_vote_with_invalid_mint_error() {
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

    // Try to use Council Mint with Community Proposal
    proposal_cookie.account.governing_token_mint =
        realm_cookie.account.config.council_mint.unwrap();

    // Act
    let err = governance_test
        .with_cast_vote(&proposal_cookie, &token_owner_record_cookie, YesNoVote::Yes)
        .await
        .err()
        .unwrap();

    assert_eq!(err, GovernanceError::InvalidGoverningMintForProposal.into());
}

#[tokio::test]
async fn test_cast_vote_with_invalid_token_owner_record_mint_error() {
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

    let proposal_cookie = governance_test
        .with_signed_off_proposal(&token_owner_record_cookie, &mut account_governance_cookie)
        .await
        .unwrap();

    // Try to use token_owner_record for Council Mint with Community Proposal
    let token_owner_record_cookie2 = governance_test
        .with_council_token_deposit(&realm_cookie)
        .await
        .unwrap();

    token_owner_record_cookie.address = token_owner_record_cookie2.address;

    // Act
    let err = governance_test
        .with_cast_vote(&proposal_cookie, &token_owner_record_cookie, YesNoVote::Yes)
        .await
        .err()
        .unwrap();

    assert_eq!(
        err,
        GovernanceError::InvalidGoverningMintForTokenOwnerRecord.into()
    );
}

#[tokio::test]
async fn test_cast_vote_with_invalid_token_owner_record_from_different_realm_error() {
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

    let proposal_cookie = governance_test
        .with_signed_off_proposal(&token_owner_record_cookie, &mut account_governance_cookie)
        .await
        .unwrap();

    // Try to use token_owner_record from another Realm for the same mint
    let realm_cookie2 = governance_test.with_realm_using_mints(&realm_cookie).await;

    let token_owner_record_cookie2 = governance_test
        .with_community_token_deposit(&realm_cookie2)
        .await
        .unwrap();

    token_owner_record_cookie.address = token_owner_record_cookie2.address;

    // Act
    let err = governance_test
        .with_cast_vote(&proposal_cookie, &token_owner_record_cookie, YesNoVote::Yes)
        .await
        .err()
        .unwrap();

    assert_eq!(err, GovernanceError::InvalidRealmForTokenOwnerRecord.into());
}

#[tokio::test]
async fn test_cast_vote_with_governance_authority_must_sign_error() {
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

    let proposal_cookie = governance_test
        .with_signed_off_proposal(&token_owner_record_cookie, &mut account_governance_cookie)
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
        .with_cast_vote(&proposal_cookie, &token_owner_record_cookie, YesNoVote::Yes)
        .await
        .err()
        .unwrap();

    assert_eq!(
        err,
        GovernanceError::GoverningTokenOwnerOrDelegateMustSign.into()
    );
}

#[tokio::test]
async fn test_cast_vote_with_vote_tipped_to_succeeded() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    let token_owner_record_cookie1 = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let mut account_governance_cookie = governance_test
        .with_account_governance(
            &realm_cookie,
            &governed_account_cookie,
            &token_owner_record_cookie1,
        )
        .await
        .unwrap();

    let token_owner_record_cookie2 = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let token_owner_record_cookie3 = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    governance_test
        .mint_community_tokens(&realm_cookie, 20)
        .await;

    let proposal_cookie = governance_test
        .with_signed_off_proposal(&token_owner_record_cookie1, &mut account_governance_cookie)
        .await
        .unwrap();

    // Act
    governance_test
        .with_cast_vote(
            &proposal_cookie,
            &token_owner_record_cookie1,
            YesNoVote::Yes,
        )
        .await
        .unwrap();

    // Assert

    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    assert_eq!(ProposalState::Voting, proposal_account.state);

    // Act
    governance_test
        .with_cast_vote(&proposal_cookie, &token_owner_record_cookie2, YesNoVote::No)
        .await
        .unwrap();

    // Assert

    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    assert_eq!(ProposalState::Voting, proposal_account.state);

    // Act
    governance_test
        .with_cast_vote(
            &proposal_cookie,
            &token_owner_record_cookie3,
            YesNoVote::Yes,
        )
        .await
        .unwrap();

    // Assert

    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    assert_eq!(ProposalState::Succeeded, proposal_account.state);

    let proposal_owner_record = governance_test
        .get_token_owner_record_account(&proposal_cookie.account.token_owner_record)
        .await;

    assert_eq!(0, proposal_owner_record.outstanding_proposal_count);
}

#[tokio::test]
async fn test_cast_vote_with_vote_tipped_to_defeated() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    // 100 votes
    let token_owner_record_cookie1 = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let mut account_governance_cookie = governance_test
        .with_account_governance(
            &realm_cookie,
            &governed_account_cookie,
            &token_owner_record_cookie1,
        )
        .await
        .unwrap();

    // 100 votes
    let token_owner_record_cookie2 = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    // 100 votes
    let token_owner_record_cookie3 = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    // Total 320 votes
    governance_test
        .mint_community_tokens(&realm_cookie, 20)
        .await;

    let proposal_cookie = governance_test
        .with_signed_off_proposal(&token_owner_record_cookie1, &mut account_governance_cookie)
        .await
        .unwrap();

    // Act
    governance_test
        .with_cast_vote(
            &proposal_cookie,
            &token_owner_record_cookie1,
            YesNoVote::Yes,
        )
        .await
        .unwrap();

    // Assert

    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    assert_eq!(ProposalState::Voting, proposal_account.state);

    // Act
    governance_test
        .with_cast_vote(&proposal_cookie, &token_owner_record_cookie2, YesNoVote::No)
        .await
        .unwrap();

    // Assert

    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    assert_eq!(ProposalState::Voting, proposal_account.state);

    // Act
    governance_test
        .with_cast_vote(&proposal_cookie, &token_owner_record_cookie3, YesNoVote::No)
        .await
        .unwrap();

    // Assert

    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    assert_eq!(ProposalState::Defeated, proposal_account.state);

    let proposal_owner_record = governance_test
        .get_token_owner_record_account(&proposal_cookie.account.token_owner_record)
        .await;

    assert_eq!(0, proposal_owner_record.outstanding_proposal_count);
}

#[tokio::test]
async fn test_cast_vote_with_threshold_below_50_and_vote_not_tipped() {
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

    // Act
    governance_test
        .with_cast_vote(&proposal_cookie, &token_owner_record_cookie, YesNoVote::Yes)
        .await
        .unwrap();

    // Assert

    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    assert_eq!(ProposalState::Voting, proposal_account.state);

    let proposal_owner_record = governance_test
        .get_token_owner_record_account(&proposal_cookie.account.token_owner_record)
        .await;

    assert_eq!(1, proposal_owner_record.outstanding_proposal_count);
}

#[tokio::test]
async fn test_cast_vote_with_voting_time_expired_error() {
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

    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    let vote_expired_at = proposal_account.voting_at.unwrap()
        + account_governance_cookie.account.config.max_voting_time as i64;

    governance_test
        .advance_clock_past_timestamp(vote_expired_at)
        .await;

    // Act

    let err = governance_test
        .with_cast_vote(&proposal_cookie, &token_owner_record_cookie, YesNoVote::No)
        .await
        .err()
        .unwrap();

    // Assert

    assert_eq!(err, GovernanceError::ProposalVotingTimeExpired.into());
}

#[tokio::test]
async fn test_cast_vote_with_cast_twice_error() {
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

    governance_test
        .mint_community_tokens(&realm_cookie, 200)
        .await;

    let proposal_cookie = governance_test
        .with_signed_off_proposal(&token_owner_record_cookie, &mut account_governance_cookie)
        .await
        .unwrap();

    governance_test
        .with_cast_vote(&proposal_cookie, &token_owner_record_cookie, YesNoVote::Yes)
        .await
        .unwrap();

    governance_test.advance_clock().await;

    // Act
    let err = governance_test
        .with_cast_vote(&proposal_cookie, &token_owner_record_cookie, YesNoVote::Yes)
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(err, GovernanceError::VoteAlreadyExists.into());
}
