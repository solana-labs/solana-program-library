#![cfg(feature = "test-bpf")]
mod program_test;

use solana_program_test::*;

use program_test::*;
use spl_governance::error::GovernanceError;

#[tokio::test]
async fn test_account_governance_created() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    // Act
    let account_governance_cookie = governance_test
        .with_account_governance(&realm_cookie, &governed_account_cookie)
        .await
        .unwrap();

    // Assert
    let account_governance_account = governance_test
        .get_governance_account(&account_governance_cookie.address)
        .await;

    assert_eq!(
        account_governance_cookie.account,
        account_governance_account
    );
}

#[tokio::test]
async fn test_create_account_governance_with_invalid_realm_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let mut realm_cookie = governance_test.with_realm().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    let account_governance_cookie = governance_test
        .with_account_governance(&realm_cookie, &governed_account_cookie)
        .await
        .unwrap();

    realm_cookie.address = account_governance_cookie.address;

    // Act
    let err = governance_test
        .with_account_governance(&realm_cookie, &governed_account_cookie)
        .await
        .err()
        .unwrap();

    // Assert

    assert_eq!(err, GovernanceError::InvalidAccountType.into());
}
