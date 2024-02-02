#![cfg(feature = "test-sbf")]

mod program_test;

use {program_test::*, solana_program_test::tokio, solana_sdk::signature::Keypair};

// TODO:
// 1) Assert the authority is on the list for the given token
// Assert authority signed
// test V1 -> V2 upgrade

#[tokio::test]
async fn test_set_token_owner_record_lock() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let token_owner_record_lock_authority = Keypair::new();

    // Act
    let _token_owner_record_lock_cookie = governance_test
        .with_token_owner_record_lock(
            &token_owner_record_cookie,
            &token_owner_record_lock_authority,
        )
        .await
        .unwrap();

    // Assert
    let token_owner_record_account = governance_test
        .get_token_owner_record_account(&token_owner_record_cookie.address)
        .await;

    assert_eq!(1, token_owner_record_account.locks.len());
}
