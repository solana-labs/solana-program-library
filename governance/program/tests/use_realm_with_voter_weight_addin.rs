#![cfg(feature = "test-bpf")]

use solana_program::pubkey::Pubkey;
use solana_program_test::*;

mod program_test;

use program_test::*;
use spl_governance::{
    error::GovernanceError,
    state::vote_record::{Vote, VoteChoice},
};
use spl_governance_addin_api::voter_weight::VoterWeightAction;

#[tokio::test]
async fn test_create_governance_with_voter_weight_addin() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_with_voter_weight_addin().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    let realm_cookie = governance_test.with_realm().await;

    let mut token_owner_record_cookie =
        governance_test.with_token_owner_record(&realm_cookie).await;

    governance_test
        .with_voter_weight_addin_record(&mut token_owner_record_cookie)
        .await
        .unwrap();

    // Act
    let governance_cookie = governance_test
        .with_governance(
            &realm_cookie,
            &governed_account_cookie,
            &token_owner_record_cookie,
        )
        .await
        .unwrap();

    // // Assert
    let governance_account = governance_test
        .get_governance_account(&governance_cookie.address)
        .await;

    assert_eq!(governance_cookie.account, governance_account);
}

#[tokio::test]
async fn test_create_proposal_with_voter_weight_addin() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_with_voter_weight_addin().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    let realm_cookie = governance_test.with_realm().await;

    let mut token_owner_record_cookie =
        governance_test.with_token_owner_record(&realm_cookie).await;

    governance_test
        .with_voter_weight_addin_record(&mut token_owner_record_cookie)
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

    // Act
    let proposal_cookie = governance_test
        .with_proposal(&token_owner_record_cookie, &mut governance_cookie)
        .await
        .unwrap();

    // // Assert
    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    assert_eq!(proposal_cookie.account, proposal_account);
}

#[tokio::test]
async fn test_cast_vote_with_voter_weight_addin() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_with_voter_weight_addin().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    let realm_cookie = governance_test.with_realm().await;

    let mut token_owner_record_cookie =
        governance_test.with_token_owner_record(&realm_cookie).await;

    governance_test
        .with_voter_weight_addin_record(&mut token_owner_record_cookie)
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

    // Act
    let vote_record_cookie = governance_test
        .with_cast_vote(&proposal_cookie, &token_owner_record_cookie, YesNoVote::Yes)
        .await
        .unwrap();

    // Assert

    let vote_record_account = governance_test
        .get_vote_record_account(&vote_record_cookie.address)
        .await;

    assert_eq!(120, vote_record_account.voter_weight);
    assert_eq!(
        Vote::Approve(vec![VoteChoice {
            rank: 0,
            weight_percentage: 100
        }]),
        vote_record_account.vote
    );

    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    assert_eq!(120, proposal_account.options[0].vote_weight);
}

#[tokio::test]
async fn test_create_token_governance_with_voter_weight_addin() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_with_voter_weight_addin().await;
    let governed_token_cookie = governance_test.with_governed_token().await;

    let realm_cookie = governance_test.with_realm().await;

    let mut token_owner_record_cookie =
        governance_test.with_token_owner_record(&realm_cookie).await;

    governance_test
        .with_voter_weight_addin_record(&mut token_owner_record_cookie)
        .await
        .unwrap();

    // Act
    let token_governance_cookie = governance_test
        .with_token_governance(
            &realm_cookie,
            &governed_token_cookie,
            &token_owner_record_cookie,
        )
        .await
        .unwrap();

    // // Assert
    let token_governance_account = governance_test
        .get_governance_account(&token_governance_cookie.address)
        .await;

    assert_eq!(token_governance_cookie.account, token_governance_account);
}

#[tokio::test]
async fn test_create_mint_governance_with_voter_weight_addin() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_with_voter_weight_addin().await;
    let governed_mint_cookie = governance_test.with_governed_mint().await;

    let realm_cookie = governance_test.with_realm().await;

    let mut token_owner_record_cookie =
        governance_test.with_token_owner_record(&realm_cookie).await;

    governance_test
        .with_voter_weight_addin_record(&mut token_owner_record_cookie)
        .await
        .unwrap();

    // Act
    let mint_governance_cookie = governance_test
        .with_mint_governance(
            &realm_cookie,
            &governed_mint_cookie,
            &token_owner_record_cookie,
        )
        .await
        .unwrap();

    // // Assert
    let mint_governance_account = governance_test
        .get_governance_account(&mint_governance_cookie.address)
        .await;

    assert_eq!(mint_governance_cookie.account, mint_governance_account);
}

#[tokio::test]
async fn test_create_program_governance_with_voter_weight_addin() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_with_voter_weight_addin().await;
    let governed_program_cookie = governance_test.with_governed_program().await;

    let realm_cookie = governance_test.with_realm().await;

    let mut token_owner_record_cookie =
        governance_test.with_token_owner_record(&realm_cookie).await;

    governance_test
        .with_voter_weight_addin_record(&mut token_owner_record_cookie)
        .await
        .unwrap();

    // Act
    let program_governance_cookie = governance_test
        .with_program_governance(
            &realm_cookie,
            &governed_program_cookie,
            &token_owner_record_cookie,
        )
        .await
        .unwrap();

    // Assert
    let program_governance_account = governance_test
        .get_governance_account(&program_governance_cookie.address)
        .await;

    assert_eq!(
        program_governance_cookie.account,
        program_governance_account
    );
}

#[tokio::test]
async fn test_realm_with_voter_weight_addin_with_deposits_not_allowed() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_with_voter_weight_addin().await;
    let realm_cookie = governance_test.with_realm().await;

    // Act

    let err = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(
        err,
        GovernanceError::GoverningTokenDepositsNotAllowed.into()
    );
}

#[tokio::test]
async fn test_create_governance_with_voter_weight_action_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_with_voter_weight_addin().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    let realm_cookie = governance_test.with_realm().await;

    let mut token_owner_record_cookie =
        governance_test.with_token_owner_record(&realm_cookie).await;

    governance_test
        .with_voter_weight_addin_record_impl(
            &mut token_owner_record_cookie,
            100,
            None,
            Some(VoterWeightAction::CastVote), // Use wrong action
            None,
        )
        .await
        .unwrap();

    // Act
    let err = governance_test
        .with_governance(
            &realm_cookie,
            &governed_account_cookie,
            &token_owner_record_cookie,
        )
        .await
        .err()
        .unwrap();

    //  Assert
    assert_eq!(err, GovernanceError::VoterWeightRecordInvalidAction.into());
}

#[tokio::test]
async fn test_create_governance_with_voter_weight_expiry_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_with_voter_weight_addin().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    let realm_cookie = governance_test.with_realm().await;

    let mut token_owner_record_cookie =
        governance_test.with_token_owner_record(&realm_cookie).await;

    governance_test
        .with_voter_weight_addin_record_impl(
            &mut token_owner_record_cookie,
            100,
            Some(1), // Past slot
            None,
            None,
        )
        .await
        .unwrap();

    governance_test.advance_clock().await;

    // Act
    let err = governance_test
        .with_governance(
            &realm_cookie,
            &governed_account_cookie,
            &token_owner_record_cookie,
        )
        .await
        .err()
        .unwrap();

    //  Assert
    assert_eq!(err, GovernanceError::VoterWeightRecordExpired.into());
}

#[tokio::test]
async fn test_cast_vote_with_voter_weight_action_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_with_voter_weight_addin().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    let realm_cookie = governance_test.with_realm().await;

    let mut token_owner_record_cookie =
        governance_test.with_token_owner_record(&realm_cookie).await;

    governance_test
        .with_voter_weight_addin_record_impl(&mut token_owner_record_cookie, 100, None, None, None)
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

    governance_test.advance_clock().await;

    governance_test
        .with_voter_weight_addin_record_impl(
            &mut token_owner_record_cookie,
            100,
            None,
            Some(VoterWeightAction::CreateGovernance), // Use wrong action
            None,
        )
        .await
        .unwrap();

    // Act

    let err = governance_test
        .with_cast_vote(&proposal_cookie, &token_owner_record_cookie, YesNoVote::Yes)
        .await
        .err()
        .unwrap();

    //  Assert
    assert_eq!(err, GovernanceError::VoterWeightRecordInvalidAction.into());
}

#[tokio::test]
async fn test_create_governance_with_voter_weight_action_target_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_with_voter_weight_addin().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    let realm_cookie = governance_test.with_realm().await;

    let mut token_owner_record_cookie =
        governance_test.with_token_owner_record(&realm_cookie).await;

    governance_test
        .with_voter_weight_addin_record_impl(
            &mut token_owner_record_cookie,
            100,
            None,
            None,
            Some(Pubkey::new_unique()), // Invalid target
        )
        .await
        .unwrap();

    governance_test.advance_clock().await;

    // Act
    let err = governance_test
        .with_governance(
            &realm_cookie,
            &governed_account_cookie,
            &token_owner_record_cookie,
        )
        .await
        .err()
        .unwrap();

    //  Assert
    assert_eq!(
        err,
        GovernanceError::VoterWeightRecordInvalidActionTarget.into()
    );
}

#[tokio::test]
async fn test_create_proposal_with_voter_weight_action_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_with_voter_weight_addin().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    let realm_cookie = governance_test.with_realm().await;

    let mut token_owner_record_cookie =
        governance_test.with_token_owner_record(&realm_cookie).await;

    governance_test
        .with_voter_weight_addin_record_impl(
            &mut token_owner_record_cookie,
            100,
            None,
            Some(VoterWeightAction::CreateGovernance),
            None,
        )
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

    // Act

    let err = governance_test
        .with_proposal(&token_owner_record_cookie, &mut governance_cookie)
        .await
        .err()
        .unwrap();

    //  Assert
    assert_eq!(err, GovernanceError::VoterWeightRecordInvalidAction.into());
}

#[tokio::test]
async fn test_create_governance_with_voter_weight_record() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_with_voter_weight_addin().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    let realm_cookie = governance_test.with_realm().await;

    let mut token_owner_record_cookie =
        governance_test.with_token_owner_record(&realm_cookie).await;

    governance_test.advance_clock().await;
    let clock = governance_test.bench.get_clock().await;

    governance_test
        .with_voter_weight_addin_record_impl(
            &mut token_owner_record_cookie,
            100,
            Some(clock.slot),
            Some(VoterWeightAction::CreateGovernance),
            Some(realm_cookie.address),
        )
        .await
        .unwrap();

    // Act
    let governance_cookie = governance_test
        .with_governance(
            &realm_cookie,
            &governed_account_cookie,
            &token_owner_record_cookie,
        )
        .await
        .unwrap();

    // // Assert
    let governance_account = governance_test
        .get_governance_account(&governance_cookie.address)
        .await;

    assert_eq!(governance_cookie.account, governance_account);
}
