#![cfg(feature = "test-sbf")]

use solana_program::pubkey::Pubkey;
use solana_program_test::*;

mod program_test;

use program_test::*;

use spl_governance::{
    error::GovernanceError,
    state::{realm::get_governing_token_holding_address, realm_config::GoverningTokenType},
};
use spl_governance_test_sdk::tools::{clone_keypair, NopOverride};

use crate::program_test::args::RealmSetupArgs;
use solana_sdk::signature::{Keypair, Signer};

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
async fn test_revoke_own_council_tokens() {
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
        .revoke_governing_tokens_using_instruction(
            &realm_cookie,
            &token_owner_record_cookie,
            &realm_cookie.account.config.council_mint.unwrap(),
            &token_owner_record_cookie.token_owner,
            token_owner_record_cookie
                .account
                .governing_token_deposit_amount,
            NopOverride,
            None,
        )
        .await
        .unwrap();

    // Assert

    let token_owner_record = governance_test
        .get_token_owner_record_account(&token_owner_record_cookie.address)
        .await;

    assert_eq!(token_owner_record.governing_token_deposit_amount, 0);
}

#[tokio::test]
async fn test_revoke_own_council_tokens_with_owner_must_sign_error() {
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
    let err = governance_test
        .revoke_governing_tokens_using_instruction(
            &realm_cookie,
            &token_owner_record_cookie,
            &realm_cookie.account.config.council_mint.unwrap(),
            &token_owner_record_cookie.token_owner,
            token_owner_record_cookie
                .account
                .governing_token_deposit_amount,
            |i| i.accounts[4].is_signer = false, // revoke_authority
            Some(&[]),
        )
        .await
        .err()
        .unwrap();

    // Assert

    assert_eq!(err, GovernanceError::GoverningTokenOwnerMustSign.into());
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

#[tokio::test]
async fn test_revoke_council_tokens_with_mint_authority_must_sign_error() {
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
    let err = governance_test
        .revoke_governing_tokens_using_instruction(
            &realm_cookie,
            &token_owner_record_cookie,
            &realm_cookie.account.config.council_mint.unwrap(),
            realm_cookie.council_mint_authority.as_ref().unwrap(),
            1,
            |i| i.accounts[4].is_signer = false, // mint_authority
            Some(&[]),
        )
        .await
        .err()
        .unwrap();

    // Assert

    assert_eq!(err, GovernanceError::MintAuthorityMustSign.into());
}

#[tokio::test]
async fn test_revoke_council_tokens_with_invalid_revoke_authority_error() {
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
    let err = governance_test
        .revoke_governing_tokens_using_instruction(
            &realm_cookie,
            &token_owner_record_cookie,
            &realm_cookie.account.config.council_mint.unwrap(),
            &Keypair::new(), // Try to use fake authority
            1,
            NopOverride,
            None,
        )
        .await
        .err()
        .unwrap();

    // Assert

    assert_eq!(err, GovernanceError::InvalidMintAuthority.into());
}

#[tokio::test]
async fn test_revoke_council_tokens_with_invalid_token_holding_error() {
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

    // Try to revoke from the community holding account
    let governing_token_holding_address = get_governing_token_holding_address(
        &governance_test.program_id,
        &realm_cookie.address,
        &realm_cookie.account.community_mint,
    );

    // Act
    let err = governance_test
        .revoke_governing_tokens_using_instruction(
            &realm_cookie,
            &token_owner_record_cookie,
            &realm_cookie.account.config.council_mint.unwrap(),
            realm_cookie.council_mint_authority.as_ref().unwrap(),
            1,
            |i| i.accounts[1].pubkey = governing_token_holding_address, // governing_token_holding_address
            None,
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
async fn test_revoke_council_tokens_with_other_realm_config_account_error() {
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

    // Try use other Realm config
    let realm_cookie2 = governance_test.with_realm().await;

    // Act
    let err = governance_test
        .revoke_governing_tokens_using_instruction(
            &realm_cookie,
            &token_owner_record_cookie,
            &realm_cookie.account.config.council_mint.unwrap(),
            realm_cookie.council_mint_authority.as_ref().unwrap(),
            1,
            |i| i.accounts[5].pubkey = realm_cookie2.realm_config.address, //realm_config_address
            None,
        )
        .await
        .err()
        .unwrap();

    // Assert

    assert_eq!(err, GovernanceError::InvalidRealmConfigForRealm.into());
}

#[tokio::test]
async fn test_revoke_council_tokens_with_invalid_realm_config_account_address_error() {
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

    // Try bypass config check by using none existing config account
    let realm_config_address = Pubkey::new_unique();

    // Act
    let err = governance_test
        .revoke_governing_tokens_using_instruction(
            &realm_cookie,
            &token_owner_record_cookie,
            &realm_cookie.account.config.council_mint.unwrap(),
            realm_cookie.council_mint_authority.as_ref().unwrap(),
            1,
            |i| i.accounts[5].pubkey = realm_config_address, // realm_config_address
            None,
        )
        .await
        .err()
        .unwrap();

    // Assert

    assert_eq!(err, GovernanceError::InvalidRealmConfigAddress.into());
}

#[tokio::test]
async fn test_revoke_council_tokens_with_token_owner_record_for_different_mint_error() {
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

    // Try to revoke from the community token owner record
    let token_owner_record_cookie2 = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    // Act
    let err = governance_test
        .revoke_governing_tokens_using_instruction(
            &realm_cookie,
            &token_owner_record_cookie,
            &realm_cookie.account.config.council_mint.unwrap(),
            realm_cookie.council_mint_authority.as_ref().unwrap(),
            1,
            |i| i.accounts[2].pubkey = token_owner_record_cookie2.address, // token_owner_record_address
            None,
        )
        .await
        .err()
        .unwrap();

    // Assert

    assert_eq!(
        err,
        GovernanceError::InvalidGoverningMintForTokenOwnerRecord.into()
    );
}

#[tokio::test]
async fn test_revoke_council_tokens_with_too_large_amount_error() {
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
    let err = governance_test
        .revoke_governing_tokens_using_instruction(
            &realm_cookie,
            &token_owner_record_cookie,
            &realm_cookie.account.config.council_mint.unwrap(),
            realm_cookie.council_mint_authority.as_ref().unwrap(),
            200,
            NopOverride,
            None,
        )
        .await
        .err()
        .unwrap();

    // Assert

    assert_eq!(err, GovernanceError::InvalidRevokeAmount.into());
}

#[tokio::test]
async fn test_revoke_council_tokens_with_partial_revoke_amount() {
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
        .revoke_governing_tokens_using_instruction(
            &realm_cookie,
            &token_owner_record_cookie,
            &realm_cookie.account.config.council_mint.unwrap(),
            realm_cookie.council_mint_authority.as_ref().unwrap(),
            5,
            NopOverride,
            None,
        )
        .await
        .unwrap();

    // Assert

    let token_owner_record = governance_test
        .get_token_owner_record_account(&token_owner_record_cookie.address)
        .await;

    assert_eq!(token_owner_record.governing_token_deposit_amount, 95);
}

#[tokio::test]
async fn test_revoke_council_tokens_with_community_mint_error() {
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

    // Try to use mint and authority for Community to revoke Council token
    let governing_token_mint = realm_cookie.account.community_mint;
    let governing_token_mint_authority = clone_keypair(&realm_cookie.community_mint_authority);
    let governing_token_holding_address = get_governing_token_holding_address(
        &governance_test.program_id,
        &realm_cookie.address,
        &governing_token_mint,
    );

    // Act
    let err = governance_test
        .revoke_governing_tokens_using_instruction(
            &realm_cookie,
            &token_owner_record_cookie,
            &realm_cookie.account.config.council_mint.unwrap(),
            realm_cookie.council_mint_authority.as_ref().unwrap(),
            1,
            |i| {
                i.accounts[1].pubkey = governing_token_holding_address;
                i.accounts[3].pubkey = governing_token_mint;
                i.accounts[4].pubkey = governing_token_mint_authority.pubkey();
            }, // mint_authority
            Some(&[&governing_token_mint_authority]),
        )
        .await
        .err()
        .unwrap();

    // Assert

    assert_eq!(err, GovernanceError::CannotRevokeGoverningTokens.into());
}

#[tokio::test]
async fn test_revoke_council_tokens_with_not_matching_mint_and_authority_error() {
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

    // Try to use a valid mint and authority not matching the Council mint
    let governing_token_mint = realm_cookie.account.community_mint;
    let governing_token_mint_authority = clone_keypair(&realm_cookie.community_mint_authority);

    // Act
    let err = governance_test
        .revoke_governing_tokens_using_instruction(
            &realm_cookie,
            &token_owner_record_cookie,
            &realm_cookie.account.config.council_mint.unwrap(),
            realm_cookie.council_mint_authority.as_ref().unwrap(),
            1,
            |i| {
                i.accounts[3].pubkey = governing_token_mint;
                i.accounts[4].pubkey = governing_token_mint_authority.pubkey();
            }, // mint_authority
            Some(&[&governing_token_mint_authority]),
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
