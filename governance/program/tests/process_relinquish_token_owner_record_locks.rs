#![cfg(feature = "test-sbf")]

mod program_test;

use {
    program_test::*,
    solana_program_test::tokio,
    solana_sdk::{program_error::ProgramError, signature::Keypair, signer::Signer},
    spl_governance::{
        error::GovernanceError, state::realm::SetRealmConfigItemArgs,
        tools::structs::SetConfigItemActionType,
    },
};

#[tokio::test]
async fn test_relinquish_token_owner_record_lock() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let token_owner_record_lock_authority_cookie = governance_test
        .with_community_token_owner_record_lock_authority(&realm_cookie)
        .await
        .unwrap();

    let token_owner_record_lock_cookie = governance_test
        .with_token_owner_record_lock(
            &token_owner_record_cookie,
            &token_owner_record_lock_authority_cookie,
        )
        .await
        .unwrap();

    // Act
    governance_test
        .relinquish_token_owner_record_locks(
            &token_owner_record_cookie,
            Some(&token_owner_record_lock_authority_cookie.authority),
            Some(vec![token_owner_record_lock_cookie.lock_id]),
        )
        .await
        .unwrap();

    // Assert
    let token_owner_record_account = governance_test
        .get_token_owner_record_account(&token_owner_record_cookie.address)
        .await;

    assert_eq!(0, token_owner_record_account.locks.len());
}

#[tokio::test]
async fn test_relinquish_token_owner_record_locks_with_invalid_authority_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let token_owner_record_lock_authority_cookie = governance_test
        .with_community_token_owner_record_lock_authority(&realm_cookie)
        .await
        .unwrap();

    let token_owner_record_lock_cookie = governance_test
        .with_token_owner_record_lock(
            &token_owner_record_cookie,
            &token_owner_record_lock_authority_cookie,
        )
        .await
        .unwrap();

    let token_owner_record_lock_authority = Keypair::new();

    // Act
    let err = governance_test
        .relinquish_token_owner_record_locks(
            &token_owner_record_cookie,
            Some(&token_owner_record_lock_authority),
            Some(vec![token_owner_record_lock_cookie.lock_id]),
        )
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(err, GovernanceError::TokenOwnerRecordLockNotFound.into());
}

#[tokio::test]
async fn test_relinquish_token_owner_record_locks_with_missing_authority_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let token_owner_record_lock_authority_cookie = governance_test
        .with_community_token_owner_record_lock_authority(&realm_cookie)
        .await
        .unwrap();

    let token_owner_record_lock_cookie = governance_test
        .with_token_owner_record_lock(
            &token_owner_record_cookie,
            &token_owner_record_lock_authority_cookie,
        )
        .await
        .unwrap();

    // Act
    let err = governance_test
        .relinquish_token_owner_record_locks(
            &token_owner_record_cookie,
            None,
            Some(vec![token_owner_record_lock_cookie.lock_id]),
        )
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(err, ProgramError::NotEnoughAccountKeys);
}

#[tokio::test]
async fn test_relinquish_token_owner_record_locks_with_invalid_lock_id_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let token_owner_record_lock_authority_cookie = governance_test
        .with_community_token_owner_record_lock_authority(&realm_cookie)
        .await
        .unwrap();

    governance_test
        .with_token_owner_record_lock(
            &token_owner_record_cookie,
            &token_owner_record_lock_authority_cookie,
        )
        .await
        .unwrap();

    // Act
    let err = governance_test
        .relinquish_token_owner_record_locks(
            &token_owner_record_cookie,
            Some(&token_owner_record_lock_authority_cookie.authority),
            Some(vec![0]),
        )
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(err, GovernanceError::TokenOwnerRecordLockNotFound.into());
}

#[tokio::test]
async fn test_relinquish_token_owner_record_locks_with_authority_must_sign_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let token_owner_record_lock_authority_cookie = governance_test
        .with_community_token_owner_record_lock_authority(&realm_cookie)
        .await
        .unwrap();

    let token_owner_record_lock_cookie = governance_test
        .with_token_owner_record_lock(
            &token_owner_record_cookie,
            &token_owner_record_lock_authority_cookie,
        )
        .await
        .unwrap();

    // Act
    let err = governance_test
        .relinquish_token_owner_record_locks_using_ix(
            &token_owner_record_cookie,
            Some(&token_owner_record_lock_authority_cookie.authority),
            Some(vec![token_owner_record_lock_cookie.lock_id]),
            |i| i.accounts[3].is_signer = false,
            Some(&[]),
        )
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(
        err,
        GovernanceError::TokenOwnerRecordLockAuthorityMustSign.into()
    );
}

#[tokio::test]
async fn test_relinquish_token_owner_record_locks_after_authority_revoked() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let token_owner_record_lock_authority_cookie = governance_test
        .with_community_token_owner_record_lock_authority(&realm_cookie)
        .await
        .unwrap();

    let token_owner_record_lock_cookie = governance_test
        .with_token_owner_record_lock(
            &token_owner_record_cookie,
            &token_owner_record_lock_authority_cookie,
        )
        .await
        .unwrap();

    // Revoke authority
    let args = SetRealmConfigItemArgs::TokenOwnerRecordLockAuthority {
        action: SetConfigItemActionType::Remove,
        governing_token_mint: realm_cookie.account.community_mint,
        authority: token_owner_record_lock_authority_cookie.authority.pubkey(),
    };

    governance_test
        .set_realm_config_item(&realm_cookie, args)
        .await
        .unwrap();

    // Act
    governance_test
        .relinquish_token_owner_record_locks(
            &token_owner_record_cookie,
            Some(&token_owner_record_lock_authority_cookie.authority),
            Some(vec![token_owner_record_lock_cookie.lock_id]),
        )
        .await
        .unwrap();

    // Assert
    let token_owner_record_account = governance_test
        .get_token_owner_record_account(&token_owner_record_cookie.address)
        .await;

    assert_eq!(0, token_owner_record_account.locks.len());
}

#[tokio::test]
async fn test_relinquish_token_owner_record_locks_after_authority_revoked_and_without_signature() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let token_owner_record_lock_authority_cookie = governance_test
        .with_community_token_owner_record_lock_authority(&realm_cookie)
        .await
        .unwrap();

    let token_owner_record_lock_cookie = governance_test
        .with_token_owner_record_lock(
            &token_owner_record_cookie,
            &token_owner_record_lock_authority_cookie,
        )
        .await
        .unwrap();

    // Revoke authority
    let args = SetRealmConfigItemArgs::TokenOwnerRecordLockAuthority {
        action: SetConfigItemActionType::Remove,
        governing_token_mint: realm_cookie.account.community_mint,
        authority: token_owner_record_lock_authority_cookie.authority.pubkey(),
    };

    governance_test
        .set_realm_config_item(&realm_cookie, args)
        .await
        .unwrap();

    // Act
    governance_test
        .relinquish_token_owner_record_locks_using_ix(
            &token_owner_record_cookie,
            Some(&token_owner_record_lock_authority_cookie.authority),
            Some(vec![token_owner_record_lock_cookie.lock_id]),
            |i| i.accounts[3].is_signer = false, // Remove authority signature
            Some(&[]),
        )
        .await
        .unwrap();

    // Assert
    let token_owner_record_account = governance_test
        .get_token_owner_record_account(&token_owner_record_cookie.address)
        .await;

    assert_eq!(0, token_owner_record_account.locks.len());
}

#[tokio::test]
async fn test_relinquish_expired_token_owner_record_lock() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let token_owner_record_lock_authority_cookie = governance_test
        .with_community_token_owner_record_lock_authority(&realm_cookie)
        .await
        .unwrap();

    // Set none expiring lock

    let lock_id = 100;

    governance_test
        .set_token_owner_record_lock(
            &token_owner_record_cookie,
            &token_owner_record_lock_authority_cookie,
            lock_id,
            None,
        )
        .await
        .unwrap();

    // Set another lock
    let token_owner_record_lock_authority_cookie2 = governance_test
        .with_community_token_owner_record_lock_authority(&realm_cookie)
        .await
        .unwrap();

    let token_owner_record_lock_cookie2 = governance_test
        .with_token_owner_record_lock(
            &token_owner_record_cookie,
            &token_owner_record_lock_authority_cookie2,
        )
        .await
        .unwrap();

    // And expire it
    governance_test
        .advance_clock_past_timestamp(token_owner_record_lock_cookie2.expiry.unwrap())
        .await;

    // Act
    governance_test
        .relinquish_token_owner_record_locks(
            &token_owner_record_cookie,
            Some(&token_owner_record_lock_authority_cookie2.authority),
            Some(vec![token_owner_record_lock_cookie2.lock_id]),
        )
        .await
        .unwrap();

    // Assert
    let token_owner_record_account = governance_test
        .get_token_owner_record_account(&token_owner_record_cookie.address)
        .await;

    assert_eq!(1, token_owner_record_account.locks.len());
    assert_eq!(lock_id, token_owner_record_account.locks[0].lock_id);
}

#[tokio::test]
async fn test_relinquish_token_owner_record_locks_for_expired_locks_only() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let token_owner_record_lock_authority_cookie = governance_test
        .with_community_token_owner_record_lock_authority(&realm_cookie)
        .await
        .unwrap();

    // Set none expiring lock

    let lock_id = 100;

    governance_test
        .set_token_owner_record_lock(
            &token_owner_record_cookie,
            &token_owner_record_lock_authority_cookie,
            lock_id,
            None,
        )
        .await
        .unwrap();

    // Set another lock
    let token_owner_record_lock_authority_cookie2 = governance_test
        .with_community_token_owner_record_lock_authority(&realm_cookie)
        .await
        .unwrap();

    let token_owner_record_lock_cookie2 = governance_test
        .with_token_owner_record_lock(
            &token_owner_record_cookie,
            &token_owner_record_lock_authority_cookie2,
        )
        .await
        .unwrap();

    // And expire it
    governance_test
        .advance_clock_past_timestamp(token_owner_record_lock_cookie2.expiry.unwrap())
        .await;

    // Act
    governance_test
        .relinquish_token_owner_record_locks(&token_owner_record_cookie, None, None)
        .await
        .unwrap();

    // Assert
    let token_owner_record_account = governance_test
        .get_token_owner_record_account(&token_owner_record_cookie.address)
        .await;

    assert_eq!(1, token_owner_record_account.locks.len());
    assert_eq!(lock_id, token_owner_record_account.locks[0].lock_id);
}

#[tokio::test]
async fn test_relinquish_multiple_token_owner_record_locks() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let token_owner_record_lock_authority_cookie = governance_test
        .with_community_token_owner_record_lock_authority(&realm_cookie)
        .await
        .unwrap();

    let lock_type1 = 1;

    governance_test
        .set_token_owner_record_lock(
            &token_owner_record_cookie,
            &token_owner_record_lock_authority_cookie,
            lock_type1,
            None,
        )
        .await
        .unwrap();

    let lock_type2 = 2;
    governance_test
        .set_token_owner_record_lock(
            &token_owner_record_cookie,
            &token_owner_record_lock_authority_cookie,
            lock_type2,
            None,
        )
        .await
        .unwrap();

    // Act
    governance_test
        .relinquish_token_owner_record_locks(
            &token_owner_record_cookie,
            Some(&token_owner_record_lock_authority_cookie.authority),
            Some(vec![lock_type1, lock_type2]),
        )
        .await
        .unwrap();

    // Assert
    let token_owner_record_account = governance_test
        .get_token_owner_record_account(&token_owner_record_cookie.address)
        .await;

    assert_eq!(0, token_owner_record_account.locks.len());
}
