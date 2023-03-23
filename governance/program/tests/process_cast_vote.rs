#![cfg(feature = "test-sbf")]

mod program_test;

use solana_program::pubkey::Pubkey;
use solana_program_test::tokio;

use program_test::*;
use spl_governance::{
    error::GovernanceError,
    state::{
        enums::{MintMaxVoterWeightSource, ProposalState, VoteThreshold, VoteTipping},
        vote_record::Vote,
    },
};

use crate::program_test::args::RealmSetupArgs;

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

    let mut governance_cookie = governance_test
        .with_governance(
            &realm_cookie,
            &governed_account_cookie,
            &token_owner_record_cookie,
        )
        .await
        .unwrap();

    let proposal_cookie = governance_test
        .with_signed_off_proposal(&token_owner_record_cookie, &mut governance_cookie)
        .await
        .unwrap();

    let clock = governance_test.bench.get_clock().await;

    // Act
    let vote_record_cookie = governance_test
        .with_cast_yes_no_vote(&proposal_cookie, &token_owner_record_cookie, YesNoVote::Yes)
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
        Some(governance_cookie.account.config.community_vote_threshold),
        proposal_account.vote_threshold
    );

    let token_owner_record = governance_test
        .get_token_owner_record_account(&token_owner_record_cookie.address)
        .await;

    assert_eq!(1, token_owner_record.unrelinquished_votes_count);

    let governance_account = governance_test
        .get_governance_account(&governance_cookie.address)
        .await;

    assert_eq!(0, governance_account.active_proposal_count);
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

    let mut governance_cookie = governance_test
        .with_governance(
            &realm_cookie,
            &governed_account_cookie,
            &token_owner_record_cookie,
        )
        .await
        .unwrap();

    let mut proposal_cookie = governance_test
        .with_signed_off_proposal(&token_owner_record_cookie, &mut governance_cookie)
        .await
        .unwrap();

    // Setup Governance for a different account
    let governed_account_cookie2 = governance_test.with_governed_account().await;

    let governance_cookie2 = governance_test
        .with_governance(
            &realm_cookie,
            &governed_account_cookie2,
            &token_owner_record_cookie,
        )
        .await
        .unwrap();

    proposal_cookie.account.governance = governance_cookie2.address;

    // Act
    let err = governance_test
        .with_cast_yes_no_vote(&proposal_cookie, &token_owner_record_cookie, YesNoVote::Yes)
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

    let mut token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let mut governance_cookie = governance_test
        .with_governance(
            &realm_cookie,
            &governed_account_cookie,
            &token_owner_record_cookie,
        )
        .await
        .unwrap();

    let proposal_cookie = governance_test
        .with_signed_off_proposal(&token_owner_record_cookie, &mut governance_cookie)
        .await
        .unwrap();

    // Try to use Council Mint with Community Proposal
    token_owner_record_cookie.account.governing_token_mint =
        realm_cookie.account.config.council_mint.unwrap();

    // Act
    let err = governance_test
        .with_cast_yes_no_vote(&proposal_cookie, &token_owner_record_cookie, YesNoVote::Yes)
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

    let mut governance_cookie = governance_test
        .with_governance(
            &realm_cookie,
            &governed_account_cookie,
            &token_owner_record_cookie,
        )
        .await
        .unwrap();

    let proposal_cookie = governance_test
        .with_signed_off_proposal(&token_owner_record_cookie, &mut governance_cookie)
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
        .with_cast_yes_no_vote(&proposal_cookie, &token_owner_record_cookie, YesNoVote::Yes)
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

    let mut governance_cookie = governance_test
        .with_governance(
            &realm_cookie,
            &governed_account_cookie,
            &token_owner_record_cookie,
        )
        .await
        .unwrap();

    let proposal_cookie = governance_test
        .with_signed_off_proposal(&token_owner_record_cookie, &mut governance_cookie)
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
        .with_cast_yes_no_vote(&proposal_cookie, &token_owner_record_cookie, YesNoVote::Yes)
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

    let mut governance_cookie = governance_test
        .with_governance(
            &realm_cookie,
            &governed_account_cookie,
            &token_owner_record_cookie,
        )
        .await
        .unwrap();

    let proposal_cookie = governance_test
        .with_signed_off_proposal(&token_owner_record_cookie, &mut governance_cookie)
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
        .with_cast_yes_no_vote(&proposal_cookie, &token_owner_record_cookie, YesNoVote::Yes)
        .await
        .err()
        .unwrap();

    assert_eq!(
        err,
        GovernanceError::GoverningTokenOwnerOrDelegateMustSign.into()
    );
}

#[tokio::test]
async fn test_cast_vote_with_strict_vote_tipped_to_succeeded() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    let token_owner_record_cookie1 = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let mut governance_cookie = governance_test
        .with_governance(
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
        .with_signed_off_proposal(&token_owner_record_cookie1, &mut governance_cookie)
        .await
        .unwrap();

    // Act
    governance_test
        .with_cast_yes_no_vote(
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
        .with_cast_yes_no_vote(&proposal_cookie, &token_owner_record_cookie2, YesNoVote::No)
        .await
        .unwrap();

    // Assert

    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    assert_eq!(ProposalState::Voting, proposal_account.state);

    // Act
    governance_test
        .with_cast_yes_no_vote(
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
async fn test_cast_vote_with_strict_vote_tipped_to_defeated() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    // 100 votes
    let token_owner_record_cookie1 = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let mut governance_cookie = governance_test
        .with_governance(
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
        .with_signed_off_proposal(&token_owner_record_cookie1, &mut governance_cookie)
        .await
        .unwrap();

    // Act
    governance_test
        .with_cast_yes_no_vote(
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
        .with_cast_yes_no_vote(&proposal_cookie, &token_owner_record_cookie2, YesNoVote::No)
        .await
        .unwrap();

    // Assert

    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    assert_eq!(ProposalState::Voting, proposal_account.state);

    // Act
    governance_test
        .with_cast_yes_no_vote(&proposal_cookie, &token_owner_record_cookie3, YesNoVote::No)
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
async fn test_cast_vote_with_early_vote_tipped_to_succeeded() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    let mut governance_config = governance_test.get_default_governance_config();

    governance_config.community_vote_tipping = VoteTipping::Early;
    governance_config.community_vote_threshold = VoteThreshold::YesVotePercentage(15);

    let token_owner_record_cookie1 = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let mut governance_cookie = governance_test
        .with_governance_using_config(
            &realm_cookie,
            &governed_account_cookie,
            &token_owner_record_cookie1,
            &governance_config,
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

    let token_owner_record_cookie4 = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let token_owner_record_cookie5 = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    governance_test
        .mint_community_tokens(&realm_cookie, 500) // total supply: 1000
        .await;

    // Test: tip by reaching 200 yes, 100 deny
    let proposal_cookie = governance_test
        .with_signed_off_proposal(&token_owner_record_cookie1, &mut governance_cookie)
        .await
        .unwrap();
    governance_test
        .with_cast_yes_no_vote(
            &proposal_cookie,
            &token_owner_record_cookie1,
            YesNoVote::Yes,
        )
        .await
        .unwrap();
    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;
    assert_eq!(ProposalState::Voting, proposal_account.state);

    governance_test
        .with_cast_yes_no_vote(&proposal_cookie, &token_owner_record_cookie2, YesNoVote::No)
        .await
        .unwrap();
    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;
    assert_eq!(ProposalState::Voting, proposal_account.state);

    governance_test
        .with_cast_yes_no_vote(
            &proposal_cookie,
            &token_owner_record_cookie3,
            YesNoVote::Yes,
        )
        .await
        .unwrap();
    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;
    assert_eq!(ProposalState::Succeeded, proposal_account.state);
    let proposal_owner_record = governance_test
        .get_token_owner_record_account(&proposal_cookie.account.token_owner_record)
        .await;
    assert_eq!(0, proposal_owner_record.outstanding_proposal_count);

    // Test: 200 vs 200 is above 15% yes, but does not tip yet
    let proposal_cookie = governance_test
        .with_signed_off_proposal(&token_owner_record_cookie1, &mut governance_cookie)
        .await
        .unwrap();
    governance_test
        .with_cast_yes_no_vote(
            &proposal_cookie,
            &token_owner_record_cookie1,
            YesNoVote::Yes,
        )
        .await
        .unwrap();
    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;
    assert_eq!(ProposalState::Voting, proposal_account.state);

    governance_test
        .with_cast_yes_no_vote(&proposal_cookie, &token_owner_record_cookie2, YesNoVote::No)
        .await
        .unwrap();
    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;
    assert_eq!(ProposalState::Voting, proposal_account.state);

    governance_test
        .with_cast_yes_no_vote(&proposal_cookie, &token_owner_record_cookie3, YesNoVote::No)
        .await
        .unwrap();
    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;
    assert_eq!(ProposalState::Voting, proposal_account.state);

    governance_test
        .with_cast_yes_no_vote(
            &proposal_cookie,
            &token_owner_record_cookie4,
            YesNoVote::Yes,
        )
        .await
        .unwrap();
    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;
    assert_eq!(ProposalState::Voting, proposal_account.state);

    // Act: 300 vs 200 makes it tip
    governance_test
        .with_cast_yes_no_vote(
            &proposal_cookie,
            &token_owner_record_cookie5,
            YesNoVote::Yes,
        )
        .await
        .unwrap();
    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;
    assert_eq!(ProposalState::Succeeded, proposal_account.state);
    let proposal_owner_record = governance_test
        .get_token_owner_record_account(&proposal_cookie.account.token_owner_record)
        .await;
    assert_eq!(0, proposal_owner_record.outstanding_proposal_count);

    let governance_account = governance_test
        .get_governance_account(&governance_cookie.address)
        .await;

    assert_eq!(0, governance_account.active_proposal_count);
}

#[tokio::test]
async fn test_cast_vote_with_early_vote_tipped_to_defeated() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    let mut governance_config = governance_test.get_default_governance_config();

    governance_config.community_vote_tipping = VoteTipping::Early;
    governance_config.community_vote_threshold = VoteThreshold::YesVotePercentage(40);

    // 100 votes
    let token_owner_record_cookie1 = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let mut _governance_cookie = governance_test
        .with_governance_using_config(
            &realm_cookie,
            &governed_account_cookie,
            &token_owner_record_cookie1,
            &governance_config,
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
        .with_signed_off_proposal(&token_owner_record_cookie1, &mut _governance_cookie)
        .await
        .unwrap();

    // Act
    governance_test
        .with_cast_yes_no_vote(
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
        .with_cast_yes_no_vote(&proposal_cookie, &token_owner_record_cookie2, YesNoVote::No)
        .await
        .unwrap();

    // Assert

    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    assert_eq!(ProposalState::Voting, proposal_account.state);

    // Act
    governance_test
        .with_cast_yes_no_vote(&proposal_cookie, &token_owner_record_cookie3, YesNoVote::No)
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

    governance_config.community_vote_threshold = VoteThreshold::YesVotePercentage(40);

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let mut governance_cookie = governance_test
        .with_governance_using_config(
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
        .with_signed_off_proposal(&token_owner_record_cookie, &mut governance_cookie)
        .await
        .unwrap();

    // Act
    governance_test
        .with_cast_yes_no_vote(&proposal_cookie, &token_owner_record_cookie, YesNoVote::Yes)
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

    let governance_account = governance_test
        .get_governance_account(&governance_cookie.address)
        .await;

    assert_eq!(1, governance_account.active_proposal_count);
}

#[tokio::test]
async fn test_cast_vote_with_disabled_tipping_yes_votes() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    let mut governance_config = governance_test.get_default_governance_config();

    governance_config.community_vote_tipping = VoteTipping::Disabled;
    governance_config.community_vote_threshold = VoteThreshold::YesVotePercentage(10);

    let token_owner_record_cookie1 = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let mut _governance_cookie = governance_test
        .with_governance_using_config(
            &realm_cookie,
            &governed_account_cookie,
            &token_owner_record_cookie1,
            &governance_config,
        )
        .await
        .unwrap();

    governance_test
        .mint_community_tokens(&realm_cookie, 20) // total supply: 120
        .await;
    let proposal_cookie = governance_test
        .with_signed_off_proposal(&token_owner_record_cookie1, &mut _governance_cookie)
        .await
        .unwrap();

    // Act
    governance_test
        .with_cast_yes_no_vote(
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

    // Act: no deny tipping
    let proposal_cookie = governance_test
        .with_signed_off_proposal(&token_owner_record_cookie1, &mut _governance_cookie)
        .await
        .unwrap();
    governance_test
        .with_cast_yes_no_vote(&proposal_cookie, &token_owner_record_cookie1, YesNoVote::No)
        .await
        .unwrap();

    // Assert

    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;
    assert_eq!(ProposalState::Voting, proposal_account.state);
}

#[tokio::test]
async fn test_cast_vote_with_disabled_tipping_no_votes() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    let mut governance_config = governance_test.get_default_governance_config();

    governance_config.community_vote_tipping = VoteTipping::Disabled;
    governance_config.community_vote_threshold = VoteThreshold::YesVotePercentage(10);

    let token_owner_record_cookie1 = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let mut _governance_cookie = governance_test
        .with_governance_using_config(
            &realm_cookie,
            &governed_account_cookie,
            &token_owner_record_cookie1,
            &governance_config,
        )
        .await
        .unwrap();

    governance_test
        .mint_community_tokens(&realm_cookie, 20) // total supply: 120
        .await;
    let proposal_cookie = governance_test
        .with_signed_off_proposal(&token_owner_record_cookie1, &mut _governance_cookie)
        .await
        .unwrap();

    // Act
    governance_test
        .with_cast_yes_no_vote(&proposal_cookie, &token_owner_record_cookie1, YesNoVote::No)
        .await
        .unwrap();

    // Assert

    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;
    assert_eq!(ProposalState::Voting, proposal_account.state);
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

    let mut governance_cookie = governance_test
        .with_governance(
            &realm_cookie,
            &governed_account_cookie,
            &token_owner_record_cookie,
        )
        .await
        .unwrap();

    let proposal_cookie = governance_test
        .with_signed_off_proposal(&token_owner_record_cookie, &mut governance_cookie)
        .await
        .unwrap();

    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    let vote_expired_at = proposal_account.voting_at.unwrap()
        + governance_cookie.account.config.voting_base_time as i64;

    governance_test
        .advance_clock_past_timestamp(vote_expired_at)
        .await;

    // Act

    let err = governance_test
        .with_cast_yes_no_vote(&proposal_cookie, &token_owner_record_cookie, YesNoVote::No)
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

    let mut governance_cookie = governance_test
        .with_governance(
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
        .with_signed_off_proposal(&token_owner_record_cookie, &mut governance_cookie)
        .await
        .unwrap();

    governance_test
        .with_cast_yes_no_vote(&proposal_cookie, &token_owner_record_cookie, YesNoVote::Yes)
        .await
        .unwrap();

    governance_test.advance_clock().await;

    // Act
    let err = governance_test
        .with_cast_yes_no_vote(&proposal_cookie, &token_owner_record_cookie, YesNoVote::Yes)
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(err, GovernanceError::VoteAlreadyExists.into());
}

#[tokio::test]
async fn test_cast_vote_with_invalid_proposal_owner_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let mut governance_cookie = governance_test
        .with_governance(
            &realm_cookie,
            &governed_account_cookie,
            &token_owner_record_cookie,
        )
        .await
        .unwrap();

    let mut proposal_cookie = governance_test
        .with_signed_off_proposal(&token_owner_record_cookie, &mut governance_cookie)
        .await
        .unwrap();

    // Try to use an invalid account as the proposal owner
    proposal_cookie.account.token_owner_record = Pubkey::new_unique();

    // Act
    let err = governance_test
        .with_cast_yes_no_vote(&proposal_cookie, &token_owner_record_cookie, YesNoVote::Yes)
        .await
        .err()
        .unwrap();

    assert_eq!(err, GovernanceError::InvalidProposalOwnerAccount.into());
}

#[tokio::test]
async fn test_cast_tipping_vote_with_invalid_proposal_owner_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let mut governance_cookie = governance_test
        .with_governance(
            &realm_cookie,
            &governed_account_cookie,
            &token_owner_record_cookie,
        )
        .await
        .unwrap();

    let mut proposal_cookie = governance_test
        .with_signed_off_proposal(&token_owner_record_cookie, &mut governance_cookie)
        .await
        .unwrap();

    // Create another voter and vote
    let token_owner_record_cookie2 = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    governance_test
        .with_cast_yes_no_vote(
            &proposal_cookie,
            &token_owner_record_cookie2,
            YesNoVote::Yes,
        )
        .await
        .unwrap();

    // Try to use the other voter as the proposal owner
    proposal_cookie.account.token_owner_record = token_owner_record_cookie2.address;

    // Act
    let err = governance_test
        .with_cast_yes_no_vote(&proposal_cookie, &token_owner_record_cookie, YesNoVote::Yes)
        .await
        .err()
        .unwrap();

    assert_eq!(err, GovernanceError::InvalidProposalOwnerAccount.into());
}

#[tokio::test]
async fn test_cast_council_vote() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    let token_owner_record_cookie = governance_test
        .with_council_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let mut governance_config = governance_test.get_default_governance_config();
    governance_config.community_vote_threshold = VoteThreshold::Disabled;

    let mut governance_cookie = governance_test
        .with_governance_using_config(
            &realm_cookie,
            &governed_account_cookie,
            &token_owner_record_cookie,
            &governance_config,
        )
        .await
        .unwrap();

    let proposal_cookie = governance_test
        .with_signed_off_proposal(&token_owner_record_cookie, &mut governance_cookie)
        .await
        .unwrap();

    // Act
    governance_test
        .with_cast_yes_no_vote(&proposal_cookie, &token_owner_record_cookie, YesNoVote::Yes)
        .await
        .unwrap();

    // Assert

    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    assert_eq!(proposal_account.state, ProposalState::Succeeded);

    assert_eq!(
        Some(governance_cookie.account.config.council_vote_threshold),
        proposal_account.vote_threshold
    );
}

#[tokio::test]
async fn test_cast_vote_with_invalid_realm_config_account_address_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let mut governance_cookie = governance_test
        .with_governance(
            &realm_cookie,
            &governed_account_cookie,
            &token_owner_record_cookie,
        )
        .await
        .unwrap();

    let proposal_cookie = governance_test
        .with_signed_off_proposal(&token_owner_record_cookie, &mut governance_cookie)
        .await
        .unwrap();

    // Try bypass config check by using none existing config account
    let realm_config_address = Pubkey::new_unique();

    // Act
    let err = governance_test
        .with_cast_vote_using_instruction(
            &proposal_cookie,
            &token_owner_record_cookie,
            Vote::Deny,
            |i| {
                i.accounts[10].pubkey = realm_config_address; // realm_config_address
            },
            None,
        )
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(err, GovernanceError::InvalidRealmConfigAddress.into());
}

#[tokio::test]
async fn test_cast_early_council_vote_with_disabled_community_vote_tipping() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    let token_owner_record_cookie = governance_test
        .with_council_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let mut governance_config = governance_test.get_default_governance_config();

    governance_config.community_vote_tipping = VoteTipping::Disabled;
    governance_config.council_vote_tipping = VoteTipping::Early;

    let mut governance_cookie = governance_test
        .with_governance_using_config(
            &realm_cookie,
            &governed_account_cookie,
            &token_owner_record_cookie,
            &governance_config,
        )
        .await
        .unwrap();

    let proposal_cookie = governance_test
        .with_signed_off_proposal(&token_owner_record_cookie, &mut governance_cookie)
        .await
        .unwrap();

    governance_test
        .mint_community_tokens(&realm_cookie, 20)
        .await;

    // Act
    governance_test
        .with_cast_yes_no_vote(&proposal_cookie, &token_owner_record_cookie, YesNoVote::Yes)
        .await
        .unwrap();

    // Assert

    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    assert_eq!(proposal_account.state, ProposalState::Succeeded);
}

#[tokio::test]
async fn test_cast_community_vote_with_early_council_and_disabled_community_vote_tipping() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let mut governance_config = governance_test.get_default_governance_config();

    governance_config.community_vote_tipping = VoteTipping::Disabled;
    governance_config.council_vote_tipping = VoteTipping::Early;

    let mut governance_cookie = governance_test
        .with_governance_using_config(
            &realm_cookie,
            &governed_account_cookie,
            &token_owner_record_cookie,
            &governance_config,
        )
        .await
        .unwrap();

    let proposal_cookie = governance_test
        .with_signed_off_proposal(&token_owner_record_cookie, &mut governance_cookie)
        .await
        .unwrap();

    governance_test
        .mint_community_tokens(&realm_cookie, 20)
        .await;

    // Act
    governance_test
        .with_cast_yes_no_vote(&proposal_cookie, &token_owner_record_cookie, YesNoVote::Yes)
        .await
        .unwrap();

    // Assert

    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    assert_eq!(proposal_account.state, ProposalState::Voting);
}

#[tokio::test]
async fn test_cast_vote_with_disabled_tipping_and_max_yes_votes() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    let mut governance_config = governance_test.get_default_governance_config();

    governance_config.community_vote_tipping = VoteTipping::Disabled;
    governance_config.community_vote_threshold = VoteThreshold::YesVotePercentage(10);

    let token_owner_record_cookie1 = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let mut _governance_cookie = governance_test
        .with_governance_using_config(
            &realm_cookie,
            &governed_account_cookie,
            &token_owner_record_cookie1,
            &governance_config,
        )
        .await
        .unwrap();

    let proposal_cookie = governance_test
        .with_signed_off_proposal(&token_owner_record_cookie1, &mut _governance_cookie)
        .await
        .unwrap();

    // Act
    governance_test
        .with_cast_yes_no_vote(
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
}

#[tokio::test]
async fn test_cast_vote_with_disabled_tipping_and_max_no_votes() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    let mut governance_config = governance_test.get_default_governance_config();

    governance_config.community_vote_tipping = VoteTipping::Disabled;
    governance_config.community_vote_threshold = VoteThreshold::YesVotePercentage(10);

    let token_owner_record_cookie1 = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let mut _governance_cookie = governance_test
        .with_governance_using_config(
            &realm_cookie,
            &governed_account_cookie,
            &token_owner_record_cookie1,
            &governance_config,
        )
        .await
        .unwrap();

    let proposal_cookie = governance_test
        .with_signed_off_proposal(&token_owner_record_cookie1, &mut _governance_cookie)
        .await
        .unwrap();

    // Act
    governance_test
        .with_cast_yes_no_vote(&proposal_cookie, &token_owner_record_cookie1, YesNoVote::No)
        .await
        .unwrap();

    // Assert

    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;
    assert_eq!(ProposalState::Voting, proposal_account.state);
}

#[tokio::test]
async fn test_cast_vote_with_strict_tipping_and_inflated_max_vote_weight() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    // Reduce max voter weight to 50% for the cast vote to be above max_voter_weight
    let realm_config_args = RealmSetupArgs {
        community_mint_max_voter_weight_source: MintMaxVoterWeightSource::SupplyFraction(
            MintMaxVoterWeightSource::SUPPLY_FRACTION_BASE / 2,
        ),
        ..Default::default()
    };

    let realm_cookie = governance_test
        .with_realm_using_args(&realm_config_args)
        .await;

    let governed_account_cookie = governance_test.with_governed_account().await;

    let governance_config = governance_test.get_default_governance_config();

    // Mint and deposit 100 tokens to Member
    let token_owner_record_cookie1 = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    // Mint 20 community tokens to increase total supply to 120
    // It gives us max_voter_weight==60 which is below the cast vote weight of 100
    governance_test
        .mint_community_tokens(&realm_cookie, 20)
        .await;

    let mut _governance_cookie = governance_test
        .with_governance_using_config(
            &realm_cookie,
            &governed_account_cookie,
            &token_owner_record_cookie1,
            &governance_config,
        )
        .await
        .unwrap();

    let proposal_cookie = governance_test
        .with_signed_off_proposal(&token_owner_record_cookie1, &mut _governance_cookie)
        .await
        .unwrap();

    // Act
    governance_test
        .with_cast_yes_no_vote(
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

    assert_eq!(ProposalState::Succeeded, proposal_account.state);
    // max_vote_weight should be coerced from 60 to 100
    assert_eq!(proposal_account.max_vote_weight, Some(100))
}

#[tokio::test]
async fn test_cast_approve_vote_with_cannot_vote_in_cool_off_time_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    // Set none default voting cool off time
    let mut governance_config = governance_test.get_default_governance_config();
    governance_config.voting_cool_off_time = 50;

    let mut governance_cookie = governance_test
        .with_governance_using_config(
            &realm_cookie,
            &governed_account_cookie,
            &token_owner_record_cookie,
            &governance_config,
        )
        .await
        .unwrap();

    let proposal_cookie = governance_test
        .with_signed_off_proposal(&token_owner_record_cookie, &mut governance_cookie)
        .await
        .unwrap();

    // Advance timestamp into voting_cool_off_time
    let clock = governance_test.bench.get_clock().await;

    governance_test
        .advance_clock_past_timestamp(
            clock.unix_timestamp + governance_cookie.account.config.voting_base_time as i64,
        )
        .await;

    // Act

    let err = governance_test
        .with_cast_yes_no_vote(&proposal_cookie, &token_owner_record_cookie, YesNoVote::Yes)
        .await
        .err()
        .unwrap();

    // Assert

    assert_eq!(err, GovernanceError::VoteNotAllowedInCoolOffTime.into());
}
