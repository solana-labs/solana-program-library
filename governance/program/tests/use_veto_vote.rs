#![cfg(feature = "test-sbf")]

mod program_test;
use program_test::*;

use solana_program_test::tokio;

use spl_governance::{
    error::GovernanceError,
    state::{
        enums::{ProposalState, VoteThreshold},
        vote_record::Vote,
    },
};

use crate::program_test::args::RealmSetupArgs;

#[tokio::test]
async fn test_cast_council_veto_vote() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    let token_owner_record_cookie = governance_test
        .with_council_token_deposit(&realm_cookie)
        .await
        .unwrap();

    // Mint extra council tokens for total supply of 120
    governance_test.mint_council_tokens(&realm_cookie, 20).await;

    let mut governance_cookie = governance_test
        .with_governance(
            &realm_cookie,
            &governed_account_cookie,
            &token_owner_record_cookie,
        )
        .await
        .unwrap();

    let proposal_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let proposal_cookie = governance_test
        .with_signed_off_proposal(&proposal_owner_record_cookie, &mut governance_cookie)
        .await
        .unwrap();

    let clock = governance_test.bench.get_clock().await;

    // Act
    let vote_record_cookie = governance_test
        .with_cast_vote(&proposal_cookie, &token_owner_record_cookie, Vote::Veto)
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
        proposal_account.veto_vote_weight
    );

    assert_eq!(proposal_account.state, ProposalState::Vetoed);
    assert_eq!(
        proposal_account.voting_completed_at,
        Some(clock.unix_timestamp)
    );

    assert_eq!(Some(120), proposal_account.max_vote_weight);
    assert_eq!(
        Some(governance_cookie.account.config.council_veto_vote_threshold),
        proposal_account.vote_threshold
    );

    let token_owner_record = governance_test
        .get_token_owner_record_account(&token_owner_record_cookie.address)
        .await;

    assert_eq!(1, token_owner_record.unrelinquished_votes_count);
    assert_eq!(1, token_owner_record.total_votes_count);

    let realm_account = governance_test
        .get_realm_account(&realm_cookie.address)
        .await;

    assert_eq!(0, realm_account.voting_proposal_count);

    let governance_account = governance_test
        .get_governance_account(&governance_cookie.address)
        .await;

    assert_eq!(0, governance_account.voting_proposal_count);
}

#[tokio::test]
async fn test_cast_community_veto_vote() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    // Mint extra community tokens for total supply of 120
    governance_test
        .mint_community_tokens(&realm_cookie, 20)
        .await;

    let mut governance_cookie = governance_test
        .with_governance(
            &realm_cookie,
            &governed_account_cookie,
            &token_owner_record_cookie,
        )
        .await
        .unwrap();

    let proposal_owner_record_cookie = governance_test
        .with_council_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let proposal_cookie = governance_test
        .with_signed_off_proposal(&proposal_owner_record_cookie, &mut governance_cookie)
        .await
        .unwrap();

    // Act
    let vote_record_cookie = governance_test
        .with_cast_vote(&proposal_cookie, &token_owner_record_cookie, Vote::Veto)
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
        proposal_account.veto_vote_weight
    );

    assert_eq!(proposal_account.state, ProposalState::Vetoed);

    assert_eq!(Some(120), proposal_account.max_vote_weight);
    assert_eq!(
        Some(
            governance_cookie
                .account
                .config
                .community_veto_vote_threshold
        ),
        proposal_account.vote_threshold
    );
}

#[tokio::test]
async fn test_cast_community_veto_vote_with_community_veto_disabled_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let mut governance_config = governance_test.get_default_governance_config();
    governance_config.community_veto_vote_threshold = VoteThreshold::Disabled;

    let mut governance_cookie = governance_test
        .with_governance_using_config(
            &realm_cookie,
            &governed_account_cookie,
            &token_owner_record_cookie,
            &governance_config,
        )
        .await
        .unwrap();

    let proposal_owner_record_cookie = governance_test
        .with_council_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let proposal_cookie = governance_test
        .with_signed_off_proposal(&proposal_owner_record_cookie, &mut governance_cookie)
        .await
        .unwrap();

    // Act
    let err = governance_test
        .with_cast_vote(&proposal_cookie, &token_owner_record_cookie, Vote::Veto)
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(
        err,
        GovernanceError::GoverningTokenMintNotAllowedToVote.into()
    );
}

#[tokio::test]
async fn test_cast_veto_vote_with_invalid_voting_mint_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    let token_owner_record_cookie = governance_test
        .with_council_token_deposit(&realm_cookie)
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

    let proposal_owner_record_cookie = governance_test
        .with_council_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let proposal_cookie = governance_test
        .with_signed_off_proposal(&proposal_owner_record_cookie, &mut governance_cookie)
        .await
        .unwrap();

    // Act

    // Try to use Council Veto on Council vote Proposal
    let err = governance_test
        .with_cast_vote(&proposal_cookie, &token_owner_record_cookie, Vote::Veto)
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(err, GovernanceError::InvalidGoverningMintForProposal.into());
}

#[tokio::test]
async fn test_cast_veto_vote_with_council_veto_vote_disabled_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    let token_owner_record_cookie = governance_test
        .with_council_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let mut governance_config = governance_test.get_default_governance_config();
    governance_config.council_veto_vote_threshold = VoteThreshold::Disabled;

    let mut governance_cookie = governance_test
        .with_governance_using_config(
            &realm_cookie,
            &governed_account_cookie,
            &token_owner_record_cookie,
            &governance_config,
        )
        .await
        .unwrap();

    let proposal_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let proposal_cookie = governance_test
        .with_signed_off_proposal(&proposal_owner_record_cookie, &mut governance_cookie)
        .await
        .unwrap();

    // Act
    let err = governance_test
        .with_cast_vote(&proposal_cookie, &token_owner_record_cookie, Vote::Veto)
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(
        err,
        GovernanceError::GoverningTokenMintNotAllowedToVote.into()
    );
}

#[tokio::test]
async fn test_cast_veto_vote_without_tipping() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    let token_owner_record_cookie = governance_test
        .with_council_token_deposit(&realm_cookie)
        .await
        .unwrap();

    // Mint extra council tokens for total supply of 201 to prevent tipping
    governance_test
        .mint_council_tokens(&realm_cookie, 101)
        .await;

    let mut governance_cookie = governance_test
        .with_governance(
            &realm_cookie,
            &governed_account_cookie,
            &token_owner_record_cookie,
        )
        .await
        .unwrap();

    let proposal_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let proposal_cookie = governance_test
        .with_signed_off_proposal(&proposal_owner_record_cookie, &mut governance_cookie)
        .await
        .unwrap();

    // Act
    let vote_record_cookie = governance_test
        .with_cast_vote(&proposal_cookie, &token_owner_record_cookie, Vote::Veto)
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
        proposal_account.veto_vote_weight
    );

    assert_eq!(proposal_account.state, ProposalState::Voting);
}

#[tokio::test]
async fn test_cast_multiple_veto_votes_for_partially_approved_proposal() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    let token_owner_record_cookie = governance_test
        .with_council_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let token_owner_record_cookie2 = governance_test
        .with_council_token_deposit(&realm_cookie)
        .await
        .unwrap();

    // Mint extra council tokens for total supply of 210 to prevent single vote tipping
    governance_test.mint_council_tokens(&realm_cookie, 10).await;

    let mut governance_cookie = governance_test
        .with_governance(
            &realm_cookie,
            &governed_account_cookie,
            &token_owner_record_cookie,
        )
        .await
        .unwrap();

    let proposal_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    // Mint extra council tokens for total supply of 200 to prevent single vote tipping
    governance_test
        .mint_community_tokens(&realm_cookie, 100)
        .await;

    let proposal_cookie = governance_test
        .with_signed_off_proposal(&proposal_owner_record_cookie, &mut governance_cookie)
        .await
        .unwrap();

    // Partially approve Proposal
    governance_test
        .with_cast_yes_no_vote(
            &proposal_cookie,
            &proposal_owner_record_cookie,
            YesNoVote::Yes,
        )
        .await
        .unwrap();

    // Partially Veto Proposal
    governance_test
        .with_cast_vote(&proposal_cookie, &token_owner_record_cookie, Vote::Veto)
        .await
        .unwrap();

    // Act

    governance_test
        .with_cast_vote(&proposal_cookie, &token_owner_record_cookie2, Vote::Veto)
        .await
        .unwrap();

    // Assert

    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    assert_eq!(200, proposal_account.veto_vote_weight);

    assert_eq!(proposal_account.state, ProposalState::Vetoed);
}

#[tokio::test]
async fn test_cast_veto_vote_with_no_council_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let mut realm_cookie = governance_test.with_realm().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    let token_owner_record_cookie = governance_test
        .with_council_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let mut governance_config = governance_test.get_default_governance_config();
    governance_config.council_veto_vote_threshold = VoteThreshold::Disabled;

    let mut governance_cookie = governance_test
        .with_governance_using_config(
            &realm_cookie,
            &governed_account_cookie,
            &token_owner_record_cookie,
            &governance_config,
        )
        .await
        .unwrap();

    let proposal_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let proposal_cookie = governance_test
        .with_signed_off_proposal(&proposal_owner_record_cookie, &mut governance_cookie)
        .await
        .unwrap();

    // Remove Council
    let realm_setup_args = RealmSetupArgs {
        use_council_mint: false,
        ..Default::default()
    };

    governance_test
        .set_realm_config(&mut realm_cookie, &realm_setup_args)
        .await
        .unwrap();

    // Act
    let err = governance_test
        .with_cast_vote(&proposal_cookie, &token_owner_record_cookie, Vote::Veto)
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(err, GovernanceError::InvalidGoverningTokenMint.into());
}
