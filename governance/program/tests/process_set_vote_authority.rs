#![cfg(feature = "test-bpf")]

use solana_program::instruction::AccountMeta;
use solana_program_test::*;

mod program_test;

use program_test::*;
use solana_sdk::signature::{Keypair, Signer};
use spl_governance::{error::GovernanceError, instruction::set_vote_authority};

#[tokio::test]
async fn test_set_community_vote_authority() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;
    let realm_cookie = governance_test.with_realm().await;
    let mut voter_record_cookie = governance_test
        .with_initial_community_token_deposit(&realm_cookie)
        .await;

    // Act
    governance_test
        .with_community_vote_authority(&realm_cookie, &mut voter_record_cookie)
        .await;

    // Assert
    let voter_record = governance_test
        .get_voter_record_account(&voter_record_cookie.address)
        .await;

    assert_eq!(
        Some(voter_record_cookie.vote_authority.pubkey()),
        voter_record.vote_authority
    );
}

#[tokio::test]
async fn test_set_vote_authority_to_none() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;
    let realm_cookie = governance_test.with_realm().await;
    let mut voter_record_cookie = governance_test
        .with_initial_community_token_deposit(&realm_cookie)
        .await;

    governance_test
        .with_community_vote_authority(&realm_cookie, &mut voter_record_cookie)
        .await;

    // Act
    governance_test
        .set_vote_authority(
            &realm_cookie,
            &voter_record_cookie,
            &voter_record_cookie.token_owner,
            &realm_cookie.account.community_mint,
            &None,
        )
        .await;

    // Assert
    let voter_record = governance_test
        .get_voter_record_account(&voter_record_cookie.address)
        .await;

    assert_eq!(None, voter_record.vote_authority);
}

#[tokio::test]
async fn test_set_council_vote_authority() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;
    let realm_cookie = governance_test.with_realm().await;
    let mut voter_record_cookie = governance_test
        .with_initial_council_token_deposit(&realm_cookie)
        .await;

    // Act
    governance_test
        .with_council_vote_authority(&realm_cookie, &mut voter_record_cookie)
        .await;

    // Assert
    let voter_record = governance_test
        .get_voter_record_account(&voter_record_cookie.address)
        .await;

    assert_eq!(
        Some(voter_record_cookie.vote_authority.pubkey()),
        voter_record.vote_authority
    );
}

#[tokio::test]
async fn test_set_community_vote_authority_with_owner_must_sign_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;
    let realm_cookie = governance_test.with_realm().await;
    let voter_record_cookie = governance_test
        .with_initial_community_token_deposit(&realm_cookie)
        .await;

    let hacker_vote_authority = Keypair::new();

    let mut instruction = set_vote_authority(
        &voter_record_cookie.token_owner.pubkey(),
        &realm_cookie.address,
        &realm_cookie.account.community_mint,
        &voter_record_cookie.token_owner.pubkey(),
        &Some(hacker_vote_authority.pubkey()),
    );

    instruction.accounts[0] =
        AccountMeta::new_readonly(voter_record_cookie.token_owner.pubkey(), false);

    // Act
    let err = governance_test
        .process_transaction(&[instruction], None)
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(
        err,
        GovernanceError::GoverningTokenOwnerOrVoteAuthrotiyMustSign.into()
    );
}

#[tokio::test]
async fn test_set_community_vote_authority_signed_by_vote_authority() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;
    let realm_cookie = governance_test.with_realm().await;
    let mut voter_record_cookie = governance_test
        .with_initial_community_token_deposit(&realm_cookie)
        .await;

    governance_test
        .with_community_vote_authority(&realm_cookie, &mut voter_record_cookie)
        .await;

    let new_vote_authority = Keypair::new();

    // Act
    governance_test
        .set_vote_authority(
            &realm_cookie,
            &voter_record_cookie,
            &voter_record_cookie.vote_authority,
            &realm_cookie.account.community_mint,
            &Some(new_vote_authority.pubkey()),
        )
        .await;

    // Assert
    let voter_record = governance_test
        .get_voter_record_account(&voter_record_cookie.address)
        .await;

    assert_eq!(
        Some(new_vote_authority.pubkey()),
        voter_record.vote_authority
    );
}
