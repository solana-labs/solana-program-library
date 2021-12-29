#![cfg(feature = "test-bpf")]

use solana_program_test::*;

mod program_test;

use program_test::*;

#[tokio::test]
async fn test_update_program_metadata() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    // Act
    let program_metadata_cookie = governance_test.with_program_metadata().await;

    // Assert
    let program_metadata_account = governance_test
        .get_program_metadata_account(&program_metadata_cookie.address)
        .await;

    assert_eq!(program_metadata_cookie.account, program_metadata_account);
}

#[tokio::test]
async fn test_update_existing_program_metadata() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    governance_test.with_program_metadata().await;

    // Act
    let program_metadata_cookie = governance_test.with_program_metadata().await;

    // Assert
    let program_metadata_account = governance_test
        .get_program_metadata_account(&program_metadata_cookie.address)
        .await;

    assert_eq!(program_metadata_cookie.account, program_metadata_account);
}
