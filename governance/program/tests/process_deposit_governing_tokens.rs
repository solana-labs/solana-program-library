#![cfg(feature = "test-bpf")]

use solana_program::instruction::AccountMeta;
use solana_program_test::*;

mod program_test;

use program_test::*;
use solana_sdk::signature::{Keypair, Signer};
use spl_governance::{error::GovernanceError, instruction::deposit_governing_tokens};

#[tokio::test]
async fn test_deposit_initial_community_tokens() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;
    let realm_cookie = governance_test.with_realm().await;

    // Act
    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await;

    // Assert

    let token_owner_record = governance_test
        .get_token_owner_record_account(&token_owner_record_cookie.address)
        .await;

    assert_eq!(token_owner_record_cookie.account, token_owner_record);

    let source_account = governance_test
        .get_token_account(&token_owner_record_cookie.token_source)
        .await;

    assert_eq!(
        token_owner_record_cookie.token_source_amount
            - token_owner_record_cookie
                .account
                .governing_token_deposit_amount,
        source_account.amount
    );

    let holding_account = governance_test
        .get_token_account(&realm_cookie.community_token_holding_account)
        .await;

    assert_eq!(
        token_owner_record.governing_token_deposit_amount,
        holding_account.amount
    );
}

#[tokio::test]
async fn test_deposit_initial_council_tokens() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;
    let realm_cookie = governance_test.with_realm().await;

    let council_token_holding_account = realm_cookie.council_token_holding_account.unwrap();

    // Act
    let token_owner_record_cookie = governance_test
        .with_council_token_deposit(&realm_cookie)
        .await;

    // Assert
    let token_owner_record = governance_test
        .get_token_owner_record_account(&token_owner_record_cookie.address)
        .await;

    assert_eq!(token_owner_record_cookie.account, token_owner_record);

    let source_account = governance_test
        .get_token_account(&token_owner_record_cookie.token_source)
        .await;

    assert_eq!(
        token_owner_record_cookie.token_source_amount
            - token_owner_record_cookie
                .account
                .governing_token_deposit_amount,
        source_account.amount
    );

    let holding_account = governance_test
        .get_token_account(&council_token_holding_account)
        .await;

    assert_eq!(
        token_owner_record.governing_token_deposit_amount,
        holding_account.amount
    );
}

#[tokio::test]
async fn test_deposit_subsequent_community_tokens() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;
    let realm_cookie = governance_test.with_realm().await;

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await;

    let deposit_amount = 5;
    let total_deposit_amount = token_owner_record_cookie
        .account
        .governing_token_deposit_amount
        + deposit_amount;

    governance_test.advance_clock().await;

    // Act
    governance_test
        .with_subsequent_community_token_deposit(
            &realm_cookie,
            &token_owner_record_cookie,
            deposit_amount,
        )
        .await;

    // Assert
    let token_owner_record = governance_test
        .get_token_owner_record_account(&token_owner_record_cookie.address)
        .await;

    assert_eq!(
        total_deposit_amount,
        token_owner_record.governing_token_deposit_amount
    );

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

    let token_owner_record_cookie = governance_test
        .with_council_token_deposit(&realm_cookie)
        .await;

    let deposit_amount = 5;
    let total_deposit_amount = token_owner_record_cookie
        .account
        .governing_token_deposit_amount
        + deposit_amount;

    governance_test.advance_clock().await;

    // Act
    governance_test
        .with_subsequent_council_token_deposit(
            &realm_cookie,
            &token_owner_record_cookie,
            deposit_amount,
        )
        .await;

    // Assert
    let token_owner_record = governance_test
        .get_token_owner_record_account(&token_owner_record_cookie.address)
        .await;

    assert_eq!(
        total_deposit_amount,
        token_owner_record.governing_token_deposit_amount
    );

    let holding_account = governance_test
        .get_token_account(&council_token_holding_account)
        .await;

    assert_eq!(total_deposit_amount, holding_account.amount);
}

#[tokio::test]
async fn test_deposit_initial_community_tokens_with_owner_must_sign_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;
    let realm_cookie = governance_test.with_realm().await;

    let token_owner = Keypair::new();
    let transfer_authority = Keypair::new();
    let token_source = Keypair::new();

    governance_test
        .create_token_account_with_transfer_authority(
            &token_source,
            &realm_cookie.account.community_mint,
            &realm_cookie.community_mint_authority,
            10,
            &token_owner,
            &transfer_authority.pubkey(),
        )
        .await;

    let mut instruction = deposit_governing_tokens(
        &governance_test.program_id,
        &realm_cookie.address,
        &token_source.pubkey(),
        &token_owner.pubkey(),
        &transfer_authority.pubkey(),
        &governance_test.context.payer.pubkey(),
        &realm_cookie.account.community_mint,
    );

    instruction.accounts[3] = AccountMeta::new_readonly(token_owner.pubkey(), false);

    // // Act

    let error = governance_test
        .process_transaction(&[instruction], Some(&[&transfer_authority]))
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(error, GovernanceError::GoverningTokenOwnerMustSign.into());
}
#[tokio::test]
async fn test_deposit_initial_community_tokens_with_invalid_owner_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;
    let realm_cookie = governance_test.with_realm().await;

    let token_owner = Keypair::new();
    let transfer_authority = Keypair::new();
    let token_source = Keypair::new();

    let invalid_owner = Keypair::new();

    governance_test
        .create_token_account_with_transfer_authority(
            &token_source,
            &realm_cookie.account.community_mint,
            &realm_cookie.community_mint_authority,
            10,
            &token_owner,
            &transfer_authority.pubkey(),
        )
        .await;

    let instruction = deposit_governing_tokens(
        &governance_test.program_id,
        &realm_cookie.address,
        &token_source.pubkey(),
        &invalid_owner.pubkey(),
        &transfer_authority.pubkey(),
        &governance_test.context.payer.pubkey(),
        &realm_cookie.account.community_mint,
    );

    // // Act

    let error = governance_test
        .process_transaction(&[instruction], Some(&[&transfer_authority, &invalid_owner]))
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(error, GovernanceError::GoverningTokenOwnerMustSign.into());
}

#[tokio::test]
async fn test_deposit_community_tokens_with_malicious_holding_account_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;
    let realm_cookie = governance_test.with_realm().await;

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await;

    governance_test
        .mint_tokens(
            &realm_cookie.account.community_mint,
            &realm_cookie.community_mint_authority,
            &token_owner_record_cookie.token_source,
            50,
        )
        .await;

    let mut deposit_ix = deposit_governing_tokens(
        &governance_test.program_id,
        &realm_cookie.address,
        &token_owner_record_cookie.token_source,
        &token_owner_record_cookie.token_owner.pubkey(),
        &token_owner_record_cookie.token_owner.pubkey(),
        &governance_test.context.payer.pubkey(),
        &realm_cookie.account.community_mint,
    );

    // Try to maliciously deposit to the source
    deposit_ix.accounts[1].pubkey = token_owner_record_cookie.token_source;

    // Act

    let err = governance_test
        .process_transaction(
            &[deposit_ix],
            Some(&[&token_owner_record_cookie.token_owner]),
        )
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(
        err,
        GovernanceError::InvalidGoverningTokenHoldingAccount.into()
    );
}
