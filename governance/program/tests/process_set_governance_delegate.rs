#![cfg(feature = "test-bpf")]

use solana_program::instruction::AccountMeta;
use solana_program_test::*;

mod program_test;

use program_test::*;
use solana_sdk::signature::{Keypair, Signer};
use spl_governance::{error::GovernanceError, instruction::set_governance_delegate};

#[tokio::test]
async fn test_set_community_governance_delegate() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;
    let realm_cookie = governance_test.with_realm().await;
    let mut token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    // Act
    governance_test
        .with_community_governance_delegate(&realm_cookie, &mut token_owner_record_cookie)
        .await;

    // Assert
    let token_owner_record = governance_test
        .get_token_owner_record_account(&token_owner_record_cookie.address)
        .await;

    assert_eq!(
        Some(token_owner_record_cookie.governance_delegate.pubkey()),
        token_owner_record.governance_delegate
    );
}

#[tokio::test]
async fn test_set_governance_delegate_to_none() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;
    let realm_cookie = governance_test.with_realm().await;
    let mut token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    governance_test
        .with_community_governance_delegate(&realm_cookie, &mut token_owner_record_cookie)
        .await;

    // Act
    governance_test
        .set_governance_delegate(
            &realm_cookie,
            &token_owner_record_cookie,
            &token_owner_record_cookie.token_owner,
            &realm_cookie.account.community_mint,
            &None,
        )
        .await;

    // Assert
    let token_owner_record = governance_test
        .get_token_owner_record_account(&token_owner_record_cookie.address)
        .await;

    assert_eq!(None, token_owner_record.governance_delegate);
}

#[tokio::test]
async fn test_set_council_governance_delegate() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;
    let realm_cookie = governance_test.with_realm().await;
    let mut token_owner_record_cookie = governance_test
        .with_council_token_deposit(&realm_cookie)
        .await
        .unwrap();

    // Act
    governance_test
        .with_council_governance_delegate(&realm_cookie, &mut token_owner_record_cookie)
        .await;

    // Assert
    let token_owner_record = governance_test
        .get_token_owner_record_account(&token_owner_record_cookie.address)
        .await;

    assert_eq!(
        Some(token_owner_record_cookie.governance_delegate.pubkey()),
        token_owner_record.governance_delegate
    );
}

#[tokio::test]
async fn test_set_community_governance_delegate_with_owner_must_sign_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;
    let realm_cookie = governance_test.with_realm().await;
    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let hacker_governance_delegate = Keypair::new();

    let mut set_delegate_ix = set_governance_delegate(
        &governance_test.program_id,
        &token_owner_record_cookie.token_owner.pubkey(),
        &realm_cookie.address,
        &realm_cookie.account.community_mint,
        &token_owner_record_cookie.token_owner.pubkey(),
        &Some(hacker_governance_delegate.pubkey()),
    );

    set_delegate_ix.accounts[0] =
        AccountMeta::new_readonly(token_owner_record_cookie.token_owner.pubkey(), false);

    // Act
    let err = governance_test
        .bench
        .process_transaction(&[set_delegate_ix], None)
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
async fn test_set_community_governance_delegate_signed_by_governance_delegate() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;
    let realm_cookie = governance_test.with_realm().await;
    let mut token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    governance_test
        .with_community_governance_delegate(&realm_cookie, &mut token_owner_record_cookie)
        .await;

    let new_governance_delegate = Keypair::new();

    // Act
    governance_test
        .set_governance_delegate(
            &realm_cookie,
            &token_owner_record_cookie,
            &token_owner_record_cookie.governance_delegate,
            &realm_cookie.account.community_mint,
            &Some(new_governance_delegate.pubkey()),
        )
        .await;

    // Assert
    let token_owner_record = governance_test
        .get_token_owner_record_account(&token_owner_record_cookie.address)
        .await;

    assert_eq!(
        Some(new_governance_delegate.pubkey()),
        token_owner_record.governance_delegate
    );
}
