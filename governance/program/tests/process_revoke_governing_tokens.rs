#![cfg(feature = "test-bpf")]

use solana_program_test::*;

mod program_test;

use program_test::*;
use spl_governance::{error::GovernanceError, state::realm_config::GoverningTokenType};

use crate::program_test::args::RealmSetupArgs;

#[tokio::test]
async fn test_revoke_community_tokens() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let mut realm_config_args = RealmSetupArgs::default();
    realm_config_args.community_token_config_args.token_type = GoverningTokenType::Membership;

    let realm_cookie = governance_test
        .with_realm_using_args(&realm_config_args)
        .await;

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    // Act
    governance_test
        .revoke_community_tokens(&realm_cookie, &token_owner_record_cookie)
        .await
        .unwrap();

    // Assert

    let token_owner_record = governance_test
        .get_token_owner_record_account(&token_owner_record_cookie.address)
        .await;

    assert_eq!(token_owner_record.governing_token_deposit_amount, 0);

    let holding_account = governance_test
        .get_token_account(&realm_cookie.community_token_holding_account)
        .await;

    assert_eq!(holding_account.amount, 0);
}

#[tokio::test]
async fn test_revoke_council_tokens() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let mut realm_config_args = RealmSetupArgs::default();
    realm_config_args.council_token_config_args.token_type = GoverningTokenType::Membership;

    let realm_cookie = governance_test
        .with_realm_using_args(&realm_config_args)
        .await;

    let token_owner_record_cookie = governance_test
        .with_council_token_deposit(&realm_cookie)
        .await
        .unwrap();

    // Act
    governance_test
        .revoke_council_tokens(&realm_cookie, &token_owner_record_cookie)
        .await
        .unwrap();

    // Assert

    let token_owner_record = governance_test
        .get_token_owner_record_account(&token_owner_record_cookie.address)
        .await;

    assert_eq!(token_owner_record.governing_token_deposit_amount, 0);

    let holding_account = governance_test
        .get_token_account(&realm_cookie.council_token_holding_account.unwrap())
        .await;

    assert_eq!(holding_account.amount, 0);
}

#[tokio::test]
async fn test_revoke_community_tokens_with_cannot_revoke_liquid_token_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    // Act
    let err = governance_test
        .revoke_community_tokens(&realm_cookie, &token_owner_record_cookie)
        .await
        .err()
        .unwrap();

    // Assert

    assert_eq!(err, GovernanceError::CannotRevokeGoverningTokens.into());
}

#[tokio::test]
async fn test_revoke_community_tokens_with_cannot_revoke_dormant_token_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let mut realm_cookie = governance_test.with_realm().await;

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let mut realm_config_args = RealmSetupArgs::default();
    realm_config_args.community_token_config_args.token_type = GoverningTokenType::Dormant;

    governance_test
        .set_realm_config(&mut realm_cookie, &realm_config_args)
        .await
        .unwrap();

    // Act
    let err = governance_test
        .revoke_community_tokens(&realm_cookie, &token_owner_record_cookie)
        .await
        .err()
        .unwrap();

    // Assert

    assert_eq!(err, GovernanceError::CannotRevokeGoverningTokens.into());
}
