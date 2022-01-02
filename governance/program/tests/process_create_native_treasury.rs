#![cfg(feature = "test-bpf")]

use solana_program_test::*;

mod program_test;

use program_test::*;

#[tokio::test]
async fn test_create_native_treasury() {
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

    // Act
    let native_treasury_cookie = governance_test
        .with_native_treasury(&account_governance_cookie)
        .await;

    // Assert

    let native_treasury_account = governance_test
        .get_native_treasury_account(&native_treasury_cookie.address)
        .await;

    assert_eq!(native_treasury_cookie.account, native_treasury_account);

    let acc = governance_test
        .bench
        .get_account(&native_treasury_cookie.address)
        .await
        .unwrap();
}
