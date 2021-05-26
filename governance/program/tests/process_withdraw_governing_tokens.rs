#![cfg(feature = "test-bpf")]

use solana_program::{instruction::AccountMeta, pubkey::Pubkey};
use solana_program_test::*;

mod program_test;

use program_test::*;
use solana_sdk::signature::Signer;

use spl_governance::{
    error::GovernanceError, instruction::withdraw_governing_tokens,
    state::token_owner_record::get_token_owner_record_address,
};

#[tokio::test]
async fn test_withdraw_community_tokens() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;
    let realm_cookie = governance_test.with_realm().await;

    let token_owner_record_cookie = governance_test
        .with_initial_community_token_deposit(&realm_cookie)
        .await;

    // Act
    governance_test
        .withdraw_community_tokens(&realm_cookie, &token_owner_record_cookie)
        .await
        .unwrap();

    // Assert
    let token_owner_record = governance_test
        .get_token_owner_record_account(&token_owner_record_cookie.address)
        .await;

    assert_eq!(0, token_owner_record.governing_token_deposit_amount);

    let holding_account = governance_test
        .get_token_account(&realm_cookie.community_token_holding_account)
        .await;

    assert_eq!(0, holding_account.amount);

    let source_account = governance_test
        .get_token_account(&token_owner_record_cookie.token_source)
        .await;

    assert_eq!(
        token_owner_record_cookie.token_source_amount,
        source_account.amount
    );
}

#[tokio::test]
async fn test_withdraw_council_tokens() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;
    let realm_cookie = governance_test.with_realm().await;

    let token_owner_record_cookie = governance_test
        .with_initial_council_token_deposit(&realm_cookie)
        .await;

    // Act
    governance_test
        .withdraw_council_tokens(&realm_cookie, &token_owner_record_cookie)
        .await
        .unwrap();

    // Assert
    let token_owner_record = governance_test
        .get_token_owner_record_account(&token_owner_record_cookie.address)
        .await;

    assert_eq!(0, token_owner_record.governing_token_deposit_amount);

    let holding_account = governance_test
        .get_token_account(&realm_cookie.council_token_holding_account.unwrap())
        .await;

    assert_eq!(0, holding_account.amount);

    let source_account = governance_test
        .get_token_account(&token_owner_record_cookie.token_source)
        .await;

    assert_eq!(
        token_owner_record_cookie.token_source_amount,
        source_account.amount
    );
}

#[tokio::test]
async fn test_withdraw_community_tokens_with_owner_must_sign_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;
    let realm_cookie = governance_test.with_realm().await;

    let token_owner_record_cookie = governance_test
        .with_initial_community_token_deposit(&realm_cookie)
        .await;

    let hacker_token_destination = Pubkey::new_unique();

    let mut instruction = withdraw_governing_tokens(
        &realm_cookie.address,
        &hacker_token_destination,
        &token_owner_record_cookie.token_owner.pubkey(),
        &realm_cookie.account.community_mint,
    );

    instruction.accounts[3] =
        AccountMeta::new_readonly(token_owner_record_cookie.token_owner.pubkey(), false);

    // Act
    let err = governance_test
        .process_transaction(&[instruction], None)
        .await
        .err()
        .unwrap();

    // Assert

    assert_eq!(err, GovernanceError::GoverningTokenOwnerMustSign.into());
}

#[tokio::test]
async fn test_withdraw_community_tokens_with_token_owner_record_address_mismatch_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;
    let realm_cookie = governance_test.with_realm().await;

    let token_owner_record_cookie = governance_test
        .with_initial_community_token_deposit(&realm_cookie)
        .await;

    let vote_record_address = get_token_owner_record_address(
        &realm_cookie.address,
        &realm_cookie.account.community_mint,
        &token_owner_record_cookie.token_owner.pubkey(),
    );

    let hacker_record_cookie = governance_test
        .with_initial_community_token_deposit(&realm_cookie)
        .await;

    let mut instruction = withdraw_governing_tokens(
        &realm_cookie.address,
        &hacker_record_cookie.token_source,
        &hacker_record_cookie.token_owner.pubkey(),
        &realm_cookie.account.community_mint,
    );

    instruction.accounts[4] = AccountMeta::new(vote_record_address, false);

    // Act
    let err = governance_test
        .process_transaction(&[instruction], Some(&[&hacker_record_cookie.token_owner]))
        .await
        .err()
        .unwrap();

    // Assert

    assert_eq!(
        err,
        GovernanceError::InvalidTokenOwnerRecordAccountAddress.into()
    );
}
