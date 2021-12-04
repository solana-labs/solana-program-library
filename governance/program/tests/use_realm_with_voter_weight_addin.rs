#![cfg(feature = "test-bpf")]

use solana_program_test::*;

mod program_test;

use program_test::*;
use spl_governance::{
    error::GovernanceError,
    state::vote_record::{Vote, VoteChoice},
};

#[tokio::test]
async fn test_create_account_governance_with_voter_weight_addin() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_with_voter_weight_addin().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    let realm_cookie = governance_test.with_realm().await;

    let mut token_owner_record_cookie =
        governance_test.with_token_owner_record(&realm_cookie).await;

    governance_test
        .with_voter_weight_addin_deposit(&mut token_owner_record_cookie)
        .await
        .unwrap();

    // Act
    let account_governance_cookie = governance_test
        .with_account_governance(
            &realm_cookie,
            &governed_account_cookie,
            &token_owner_record_cookie,
        )
        .await
        .unwrap();

    // // Assert
    let account_governance_account = governance_test
        .get_governance_account(&account_governance_cookie.address)
        .await;

    assert_eq!(
        account_governance_cookie.account,
        account_governance_account
    );
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
        .with_voter_weight_addin_deposit(&mut token_owner_record_cookie)
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

    // Act
    let proposal_cookie = governance_test
        .with_proposal(&token_owner_record_cookie, &mut account_governance_cookie)
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
        .with_voter_weight_addin_deposit(&mut token_owner_record_cookie)
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
        .with_voter_weight_addin_deposit(&mut token_owner_record_cookie)
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
        .with_voter_weight_addin_deposit(&mut token_owner_record_cookie)
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
        .with_voter_weight_addin_deposit(&mut token_owner_record_cookie)
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
