#![cfg(feature = "test-bpf")]

use solana_program_test::*;

mod program_test;

use program_test::*;

#[tokio::test]
async fn test_program_governance_created() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_program_cookie = governance_test.with_governed_program().await;

    // Act
    let program_governance_cookie = governance_test
        .with_program_governance(&realm_cookie, &governed_program_cookie)
        .await;

    // Assert
    let program_governance_account = governance_test
        .get_program_governance_account(&program_governance_cookie.address)
        .await;

    assert_eq!(
        program_governance_cookie.account,
        program_governance_account
    );
}
