#![cfg(feature = "test-sbf")]

mod program_test;

use {
    program_test::*,
    solana_program_test::tokio,
    solana_sdk::{signature::Keypair, signer::Signer},
    spl_governance::{
        error::GovernanceError,
        state::{enums::GovernanceAccountType, legacy::TokenOwnerRecordV1},
    },
    spl_governance_tools::account::AccountMaxSize,
};

#[tokio::test]
async fn test_set_token_owner_record_lock() {
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

    // Act
    let token_owner_record_lock_cookie = governance_test
        .with_token_owner_record_lock(
            &token_owner_record_cookie,
            &token_owner_record_lock_authority_cookie,
        )
        .await
        .unwrap();

    // Assert
    let token_owner_record_account = governance_test
        .get_token_owner_record_account(&token_owner_record_cookie.address)
        .await;

    assert_eq!(1, token_owner_record_account.locks.len());
    assert_eq!(
        token_owner_record_lock_cookie.authority,
        token_owner_record_account.locks[0].authority
    );
    assert_eq!(
        token_owner_record_lock_cookie.lock_id,
        token_owner_record_account.locks[0].lock_id
    );
    assert_eq!(
        token_owner_record_lock_cookie.expiry,
        token_owner_record_account.locks[0].expiry
    );
}

#[tokio::test]
async fn test_override_existing_token_owner_record_lock() {
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

    let expiry = None;
    let lock_id = token_owner_record_lock_cookie.lock_id;

    // Act

    governance_test
        .set_token_owner_record_lock(
            &token_owner_record_cookie,
            &token_owner_record_lock_authority_cookie,
            lock_id,
            expiry,
        )
        .await
        .unwrap();

    // Assert
    let token_owner_record_account = governance_test
        .get_token_owner_record_account(&token_owner_record_cookie.address)
        .await;

    assert_eq!(1, token_owner_record_account.locks.len());
    assert_eq!(
        token_owner_record_lock_authority_cookie.authority.pubkey(),
        token_owner_record_account.locks[0].authority
    );
    assert_eq!(lock_id, token_owner_record_account.locks[0].lock_id);
    assert_eq!(expiry, token_owner_record_account.locks[0].expiry);
}

#[tokio::test]
async fn test_set_token_owner_record_lock_and_trim_expired_lock() {
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

    governance_test
        .advance_clock_past_timestamp(token_owner_record_lock_cookie.expiry.unwrap())
        .await;

    let expiry = None;
    let lock_id = 101;

    // Act

    governance_test
        .set_token_owner_record_lock(
            &token_owner_record_cookie,
            &token_owner_record_lock_authority_cookie,
            lock_id,
            expiry,
        )
        .await
        .unwrap();

    // Assert
    let token_owner_record_account = governance_test
        .get_token_owner_record_account(&token_owner_record_cookie.address)
        .await;

    assert_eq!(1, token_owner_record_account.locks.len());
    assert_eq!(
        token_owner_record_lock_authority_cookie.authority.pubkey(),
        token_owner_record_account.locks[0].authority
    );
    assert_eq!(lock_id, token_owner_record_account.locks[0].lock_id);
    assert_eq!(expiry, token_owner_record_account.locks[0].expiry);
}

#[tokio::test]
async fn test_set_multiple_token_owner_record_locks() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let token_owner_record_lock_authority_cookie1 = governance_test
        .with_community_token_owner_record_lock_authority(&realm_cookie)
        .await
        .unwrap();

    let token_owner_record_lock_authority_cookie2 = governance_test
        .with_community_token_owner_record_lock_authority(&realm_cookie)
        .await
        .unwrap();

    // Act
    let token_owner_record_lock_cookie1 = governance_test
        .with_token_owner_record_lock(
            &token_owner_record_cookie,
            &token_owner_record_lock_authority_cookie1,
        )
        .await
        .unwrap();

    let token_owner_record_lock_cookie2 = governance_test
        .with_token_owner_record_lock(
            &token_owner_record_cookie,
            &token_owner_record_lock_authority_cookie2,
        )
        .await
        .unwrap();

    // Assert
    let token_owner_record_account = governance_test
        .get_token_owner_record_account(&token_owner_record_cookie.address)
        .await;

    assert_eq!(2, token_owner_record_account.locks.len());
    assert_eq!(
        token_owner_record_lock_cookie1.authority,
        token_owner_record_account.locks[0].authority
    );
    assert_eq!(
        token_owner_record_lock_cookie2.authority,
        token_owner_record_account.locks[1].authority
    );
}

#[tokio::test]
async fn test_set_token_owner_record_lock_with_lock_authority_must_sign_error() {
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

    let expiry = None;
    let lock_id = 101;

    // Act
    let err = governance_test
        .set_token_owner_record_lock_using_ix(
            &token_owner_record_cookie,
            &token_owner_record_lock_authority_cookie,
            lock_id,
            expiry,
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
async fn test_set_token_owner_record_lock_with_invalid_lock_authority_error() {
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

    let expiry = None;
    let lock_id = 101;
    let lock_authority = Keypair::new();

    // Act
    let err = governance_test
        .set_token_owner_record_lock_using_ix(
            &token_owner_record_cookie,
            &token_owner_record_lock_authority_cookie,
            lock_id,
            expiry,
            |i| i.accounts[3].pubkey = lock_authority.pubkey(),
            Some(&[&lock_authority]),
        )
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(
        err,
        GovernanceError::InvalidTokenOwnerRecordLockAuthority.into()
    );
}

#[tokio::test]
async fn test_set_token_owner_record_lock_with_invalid_realm_config_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let realm_cookie2 = governance_test.with_realm().await;

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let token_owner_record_lock_authority_cookie = governance_test
        .with_community_token_owner_record_lock_authority(&realm_cookie)
        .await
        .unwrap();

    let expiry = None;
    let lock_id = 101;

    // Act
    let err = governance_test
        .set_token_owner_record_lock_using_ix(
            &token_owner_record_cookie,
            &token_owner_record_lock_authority_cookie,
            lock_id,
            expiry,
            |i| i.accounts[1].pubkey = realm_cookie2.realm_config.address,
            None,
        )
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(err, GovernanceError::InvalidRealmConfigForRealm.into());
}

#[tokio::test]
async fn test_set_token_owner_record_lock_with_invalid_token_owner_record_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let realm_cookie2 = governance_test.with_realm().await;

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let token_owner_record_cookie2 = governance_test
        .with_community_token_deposit(&realm_cookie2)
        .await
        .unwrap();

    let token_owner_record_lock_authority_cookie = governance_test
        .with_community_token_owner_record_lock_authority(&realm_cookie)
        .await
        .unwrap();

    let expiry = None;
    let lock_id = 101;

    // Act
    let err = governance_test
        .set_token_owner_record_lock_using_ix(
            &token_owner_record_cookie,
            &token_owner_record_lock_authority_cookie,
            lock_id,
            expiry,
            |i| i.accounts[2].pubkey = token_owner_record_cookie2.address,
            None,
        )
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(err, GovernanceError::InvalidRealmForTokenOwnerRecord.into());
}

#[tokio::test]
async fn test_set_community_token_owner_record_lock_with_council_authority_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    governance_test
        .with_community_token_owner_record_lock_authority(&realm_cookie)
        .await
        .unwrap();

    let council_token_owner_record_lock_authority_cookie = governance_test
        .with_council_token_owner_record_lock_authority(&realm_cookie)
        .await
        .unwrap();

    // Act
    let err = governance_test
        .with_token_owner_record_lock(
            &token_owner_record_cookie,
            &council_token_owner_record_lock_authority_cookie,
        )
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(
        err,
        GovernanceError::InvalidTokenOwnerRecordLockAuthority.into()
    );
}

#[tokio::test]
async fn test_set_community_token_owner_record_lock_with_expired_lock_error() {
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

    let expiry = Some(0);
    let lock_id = 101;

    // Act
    let err = governance_test
        .set_token_owner_record_lock(
            &token_owner_record_cookie,
            &token_owner_record_lock_authority_cookie,
            lock_id,
            expiry,
        )
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(err, GovernanceError::ExpiredTokenOwnerRecordLock.into());
}

#[tokio::test]
async fn test_set_token_owner_record_lock_with_extended_account_size() {
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

    let mut token_owner_record_account = governance_test
        .bench
        .get_account(&token_owner_record_cookie.address)
        .await
        .unwrap();

    let token_owner_record_account_size = token_owner_record_account.data.len();

    // Act
    governance_test
        .with_token_owner_record_lock(
            &token_owner_record_cookie,
            &token_owner_record_lock_authority_cookie,
        )
        .await
        .unwrap();

    // Assert
    token_owner_record_account = governance_test
        .bench
        .get_account(&token_owner_record_cookie.address)
        .await
        .unwrap();

    assert_eq!(
        token_owner_record_account_size + 42,
        token_owner_record_account.data.len()
    );
}

#[tokio::test]
async fn test_set_token_owner_record_lock_for_v1_account() {
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

    let token_owner_record_data_v1 = TokenOwnerRecordV1 {
        account_type: GovernanceAccountType::TokenOwnerRecordV1,
        realm: token_owner_record_cookie.account.realm,
        governing_token_mint: token_owner_record_cookie.account.governing_token_mint,
        governing_token_owner: token_owner_record_cookie.account.governing_token_owner,
        governing_token_deposit_amount: token_owner_record_cookie
            .account
            .governing_token_deposit_amount,
        governance_delegate: token_owner_record_cookie.account.governance_delegate,
        unrelinquished_votes_count: token_owner_record_cookie.account.unrelinquished_votes_count,
        outstanding_proposal_count: token_owner_record_cookie.account.outstanding_proposal_count,
        version: 0,
        reserved: [0; 6],
    };

    governance_test.bench.set_borsh_account(
        &governance_test.program_id,
        &token_owner_record_cookie.address,
        &token_owner_record_data_v1,
    );

    // Act
    governance_test
        .with_token_owner_record_lock(
            &token_owner_record_cookie,
            &token_owner_record_lock_authority_cookie,
        )
        .await
        .unwrap();

    // Assert
    let token_owner_record_data = governance_test
        .get_token_owner_record_account(&token_owner_record_cookie.address)
        .await;

    assert_eq!(
        GovernanceAccountType::TokenOwnerRecordV2,
        token_owner_record_data.account_type
    );

    let token_owner_record_account = governance_test
        .bench
        .get_account(&token_owner_record_cookie.address)
        .await
        .unwrap();

    assert_eq!(
        token_owner_record_data.get_max_size().unwrap(),
        token_owner_record_account.data.len()
    );
}

#[tokio::test]
async fn test_set_token_owner_record_lock_with_non_expiring_and_expiring_locks() {
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

    let lock_id1 = 0;
    let expiry1 = None;

    let lock_id2 = 1;
    let clock = governance_test.bench.get_clock().await;
    let expiry2 = Some(clock.unix_timestamp + 1);

    // Act

    governance_test
        .set_token_owner_record_lock(
            &token_owner_record_cookie,
            &token_owner_record_lock_authority_cookie,
            lock_id1,
            expiry1,
        )
        .await
        .unwrap();

    governance_test
        .set_token_owner_record_lock(
            &token_owner_record_cookie,
            &token_owner_record_lock_authority_cookie,
            lock_id2,
            expiry2,
        )
        .await
        .unwrap();

    // Assert
    let token_owner_record_account = governance_test
        .get_token_owner_record_account(&token_owner_record_cookie.address)
        .await;

    assert_eq!(None, token_owner_record_account.locks[0].expiry);
    assert_eq!(expiry2, token_owner_record_account.locks[1].expiry);
}

#[tokio::test]
async fn test_set_token_owner_record_lock_with_multiple_non_expiring_locks() {
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

    let lock_id1 = 0;
    let expiry1 = None;

    let lock_id2 = 1;
    let expiry2 = None;

    governance_test
        .set_token_owner_record_lock(
            &token_owner_record_cookie,
            &token_owner_record_lock_authority_cookie,
            lock_id1,
            expiry1,
        )
        .await
        .unwrap();

    governance_test
        .set_token_owner_record_lock(
            &token_owner_record_cookie,
            &token_owner_record_lock_authority_cookie,
            lock_id2,
            expiry2,
        )
        .await
        .unwrap();

    let clock = governance_test.bench.get_clock().await;
    let new_expiry1 = Some(clock.unix_timestamp + 1);

    // Act

    governance_test
        .set_token_owner_record_lock(
            &token_owner_record_cookie,
            &token_owner_record_lock_authority_cookie,
            lock_id1,
            new_expiry1,
        )
        .await
        .unwrap();

    // Assert
    let token_owner_record_account = governance_test
        .get_token_owner_record_account(&token_owner_record_cookie.address)
        .await;

    assert_eq!(new_expiry1, token_owner_record_account.locks[0].expiry);
    assert_eq!(None, token_owner_record_account.locks[1].expiry);
}
