#![cfg(feature = "test-bpf")]

use solana_program_test::*;

mod program_test;

use program_test::*;

#[tokio::test]
async fn test_account_governance_created() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    // Act
    let account_governance_cookie = governance_test
        .with_account_governance(&realm_cookie, &governed_account_cookie)
        .await;

    // Assert
    let account_governance_account = governance_test
        .get_program_governance_account(&account_governance_cookie.address)
        .await;

    assert_eq!(
        account_governance_cookie.account,
        account_governance_account
    );
}
