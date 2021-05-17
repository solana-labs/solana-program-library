#![cfg(feature = "test-bpf")]

use solana_program_test::*;

mod program_test;

use program_test::*;

#[tokio::test]
async fn test_deposit_initial_community_tokens() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;
    let realm_cookie = governance_test.with_realm().await;

    // Act
    let voter_record_cookie = governance_test
        .with_initial_community_token_deposit(&realm_cookie)
        .await;

    // Assert

    let voter_record = governance_test
        .get_voter_record_account(&voter_record_cookie.address)
        .await;

    assert_eq!(voter_record_cookie.account, voter_record);

    let source_account = governance_test
        .get_token_account(&voter_record_cookie.token_source)
        .await;

    assert_eq!(
        voter_record_cookie.token_source_amount - voter_record_cookie.account.token_deposit_amount,
        source_account.amount
    );

    let holding_account = governance_test
        .get_token_account(&realm_cookie.community_token_holding_account)
        .await;

    assert_eq!(voter_record.token_deposit_amount, holding_account.amount);
}

#[tokio::test]
async fn test_deposit_initial_council_tokens() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;
    let realm_cookie = governance_test.with_realm().await;

    let council_token_holding_account = realm_cookie.council_token_holding_account.unwrap();

    // Act
    let voter_record_cookie = governance_test
        .with_initial_council_token_deposit(&realm_cookie)
        .await;

    // Assert
    let voter_record = governance_test
        .get_voter_record_account(&voter_record_cookie.address)
        .await;

    assert_eq!(voter_record_cookie.account, voter_record);

    let source_account = governance_test
        .get_token_account(&voter_record_cookie.token_source)
        .await;

    assert_eq!(
        voter_record_cookie.token_source_amount - voter_record_cookie.account.token_deposit_amount,
        source_account.amount
    );

    let holding_account = governance_test
        .get_token_account(&council_token_holding_account)
        .await;

    assert_eq!(voter_record.token_deposit_amount, holding_account.amount);
}

#[tokio::test]
async fn test_deposit_subsequent_community_tokens() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;
    let realm_cookie = governance_test.with_realm().await;

    let voter_record_cookie = governance_test
        .with_initial_community_token_deposit(&realm_cookie)
        .await;

    let deposit_amount = 5;
    let total_deposit_amount = voter_record_cookie.account.token_deposit_amount + deposit_amount;

    // Act
    governance_test
        .with_community_token_deposit(&realm_cookie, &voter_record_cookie, deposit_amount)
        .await;

    // Assert
    let voter_record = governance_test
        .get_voter_record_account(&voter_record_cookie.address)
        .await;

    assert_eq!(total_deposit_amount, voter_record.token_deposit_amount);

    let holding_account = governance_test
        .get_token_account(&realm_cookie.community_token_holding_account)
        .await;

    assert_eq!(total_deposit_amount, holding_account.amount);
}

#[tokio::test]
async fn test_deposit_subsequent_council_tokens() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;
    let realm_cookie = governance_test.with_realm().await;

    let council_token_holding_account = realm_cookie.council_token_holding_account.unwrap();

    let voter_record_cookie = governance_test
        .with_initial_council_token_deposit(&realm_cookie)
        .await;

    let deposit_amount = 5;
    let total_deposit_amount = voter_record_cookie.account.token_deposit_amount + deposit_amount;

    // Act
    governance_test
        .with_council_token_deposit(&realm_cookie, &voter_record_cookie, deposit_amount)
        .await;

    // Assert
    let voter_record = governance_test
        .get_voter_record_account(&voter_record_cookie.address)
        .await;

    assert_eq!(total_deposit_amount, voter_record.token_deposit_amount);

    let holding_account = governance_test
        .get_token_account(&council_token_holding_account)
        .await;

    assert_eq!(total_deposit_amount, holding_account.amount);
}
