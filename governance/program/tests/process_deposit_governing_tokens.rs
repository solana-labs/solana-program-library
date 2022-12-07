#![cfg(feature = "test-sbf")]

use solana_program::instruction::AccountMeta;
use solana_program_test::*;

mod program_test;

use program_test::*;
use solana_sdk::signature::{Keypair, Signer};
use spl_governance::{
    error::GovernanceError,
    instruction::deposit_governing_tokens,
    state::{
        realm_config::GoverningTokenType, token_owner_record::TOKEN_OWNER_RECORD_LAYOUT_VERSION,
    },
};

use crate::program_test::args::*;

#[tokio::test]
async fn test_deposit_initial_community_tokens() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;
    let realm_cookie = governance_test.with_realm().await;

    // Act
    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    // Assert

    let token_owner_record = governance_test
        .get_token_owner_record_account(&token_owner_record_cookie.address)
        .await;

    assert_eq!(token_owner_record_cookie.account, token_owner_record);

    assert_eq!(
        TOKEN_OWNER_RECORD_LAYOUT_VERSION,
        token_owner_record.version
    );
    assert_eq!(0, token_owner_record.unrelinquished_votes_count);

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
        .await
        .unwrap();

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
        .await
        .unwrap();

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
        .await
        .unwrap();

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

    let amount = 10;

    governance_test
        .bench
        .create_token_account_with_transfer_authority(
            &token_source,
            &realm_cookie.account.community_mint,
            &realm_cookie.community_mint_authority,
            amount,
            &token_owner,
            &transfer_authority.pubkey(),
        )
        .await;

    let mut deposit_ix = deposit_governing_tokens(
        &governance_test.program_id,
        &realm_cookie.address,
        &token_source.pubkey(),
        &token_owner.pubkey(),
        &transfer_authority.pubkey(),
        &governance_test.bench.context.payer.pubkey(),
        amount,
        &realm_cookie.account.community_mint,
    );

    deposit_ix.accounts[3] = AccountMeta::new_readonly(token_owner.pubkey(), false);

    // Act

    let error = governance_test
        .bench
        .process_transaction(&[deposit_ix], Some(&[&transfer_authority]))
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
        .await
        .unwrap();

    let amount = 50;

    governance_test
        .bench
        .mint_tokens(
            &realm_cookie.account.community_mint,
            &realm_cookie.community_mint_authority,
            &token_owner_record_cookie.token_source,
            amount,
        )
        .await;

    let mut deposit_ix = deposit_governing_tokens(
        &governance_test.program_id,
        &realm_cookie.address,
        &token_owner_record_cookie.token_source,
        &token_owner_record_cookie.token_owner.pubkey(),
        &token_owner_record_cookie.token_owner.pubkey(),
        &governance_test.bench.context.payer.pubkey(),
        amount,
        &realm_cookie.account.community_mint,
    );

    // Try to maliciously deposit to the source
    deposit_ix.accounts[1].pubkey = token_owner_record_cookie.token_source;

    // Act

    let err = governance_test
        .bench
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

#[tokio::test]
async fn test_deposit_community_tokens_using_mint() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;
    let realm_cookie = governance_test.with_realm().await;

    // Act
    let token_owner_record_cookie = governance_test
        .with_initial_governing_token_deposit_using_mint(
            &realm_cookie.address,
            &realm_cookie.account.community_mint,
            &realm_cookie.community_mint_authority,
            10,
            None,
        )
        .await
        .unwrap();

    // Assert

    let token_owner_record = governance_test
        .get_token_owner_record_account(&token_owner_record_cookie.address)
        .await;

    assert_eq!(token_owner_record_cookie.account, token_owner_record);

    let holding_account = governance_test
        .get_token_account(&realm_cookie.community_token_holding_account)
        .await;

    assert_eq!(
        token_owner_record.governing_token_deposit_amount,
        holding_account.amount
    );
}

#[tokio::test]
async fn test_deposit_comunity_tokens_with_cannot_deposit_dormant_tokens_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let mut realm_config_args = RealmSetupArgs::default();
    realm_config_args.council_token_config_args.token_type = GoverningTokenType::Dormant;

    let realm_cookie = governance_test
        .with_realm_using_args(&realm_config_args)
        .await;

    // Act
    let err = governance_test
        .with_council_token_deposit(&realm_cookie)
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(err, GovernanceError::CannotDepositDormantTokens.into());
}
