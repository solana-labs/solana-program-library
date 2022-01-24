#![cfg(feature = "test-bpf")]

use solana_program::pubkey::Pubkey;
use solana_program_test::*;

mod program_test;

use program_test::*;
use spl_governance::error::GovernanceError;
use spl_governance_tools::error::GovernanceToolsError;

#[tokio::test]
async fn test_set_realm_authority() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;

    let governed_account_cookie = governance_test.with_governed_account().await;

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let account_governance_cookie = governance_test
        .with_account_governance(
            &realm_cookie,
            &governed_account_cookie,
            &token_owner_record_cookie,
        )
        .await
        .unwrap();

    let new_realm_authority = account_governance_cookie.address;

    // Act
    governance_test
        .set_realm_authority(&realm_cookie, &Some(new_realm_authority))
        .await
        .unwrap();

    // Assert
    let realm_account = governance_test
        .get_realm_account(&realm_cookie.address)
        .await;

    assert_eq!(realm_account.authority, Some(new_realm_authority));
}

#[tokio::test]
async fn test_set_realm_authority_with_non_existing_new_authority_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;

    let new_realm_authority = Pubkey::new_unique();

    // Act
    let err = governance_test
        .set_realm_authority(&realm_cookie, &Some(new_realm_authority))
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(err, GovernanceToolsError::AccountDoesNotExist.into());
}

#[tokio::test]
async fn test_set_realm_authority_to_none() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;

    // Act
    governance_test
        .set_realm_authority(&realm_cookie, &None)
        .await
        .unwrap();

    // Assert
    let realm_account = governance_test
        .get_realm_account(&realm_cookie.address)
        .await;

    assert_eq!(realm_account.authority, None);
}

#[tokio::test]
async fn test_set_realm_authority_with_no_authority_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;

    governance_test
        .set_realm_authority(&realm_cookie, &None)
        .await
        .unwrap();

    let new_realm_authority = Pubkey::new_unique();

    // Act
    let err = governance_test
        .set_realm_authority(&realm_cookie, &Some(new_realm_authority))
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(err, GovernanceError::RealmHasNoAuthority.into());
}

#[tokio::test]
async fn test_set_realm_authority_with_invalid_authority_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let mut realm_cookie = governance_test.with_realm().await;
    let realm_cookie2 = governance_test.with_realm().await;

    let new_realm_authority = Pubkey::new_unique();

    // Try to use authority from other realm
    realm_cookie.realm_authority = realm_cookie2.realm_authority;

    // Act
    let err = governance_test
        .set_realm_authority(&realm_cookie, &Some(new_realm_authority))
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(err, GovernanceError::InvalidAuthorityForRealm.into());
}

#[tokio::test]
async fn test_set_realm_authority_with_authority_must_sign_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;

    let new_realm_authority = Pubkey::new_unique();

    // Act
    let err = governance_test
        .set_realm_authority_using_instruction(
            &realm_cookie,
            &Some(new_realm_authority),
            |i| i.accounts[1].is_signer = false, // realm_authority
            Some(&[]),
        )
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(err, GovernanceError::RealmAuthorityMustSign.into());
}

#[tokio::test]
async fn test_set_realm_authority_with_governance_from_other_realm_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;

    // Setup other realm
    let realm_cookie2 = governance_test.with_realm().await;

    let governed_account_cookie2 = governance_test.with_governed_account().await;

    let token_owner_record_cookie2 = governance_test
        .with_community_token_deposit(&realm_cookie2)
        .await
        .unwrap();

    let account_governance_cookie2 = governance_test
        .with_account_governance(
            &realm_cookie2,
            &governed_account_cookie2,
            &token_owner_record_cookie2,
        )
        .await
        .unwrap();

    let new_realm_authority = account_governance_cookie2.address;

    // Act
    let err = governance_test
        .set_realm_authority(&realm_cookie, &Some(new_realm_authority))
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(err, GovernanceError::InvalidRealmForGovernance.into());
}
