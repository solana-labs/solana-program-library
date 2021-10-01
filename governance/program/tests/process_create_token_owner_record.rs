#![cfg(feature = "test-bpf")]

use solana_program_test::*;

mod program_test;

use program_test::*;

#[tokio::test]
async fn test_create_token_owner_record() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;
    let realm_cookie = governance_test.with_realm().await;

    // Act
    let token_owner_record_cookie = governance_test.with_token_owner_record(&realm_cookie).await;

    // Assert
    let token_owner_record_account = governance_test
        .get_token_owner_record_account(&token_owner_record_cookie.address)
        .await;

    assert_eq!(0, token_owner_record_account.governing_token_deposit_amount);

    assert_eq!(
        token_owner_record_cookie.account,
        token_owner_record_account
    );
}
