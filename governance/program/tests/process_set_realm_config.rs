#![cfg(feature = "test-bpf")]

use solana_program::pubkey::Pubkey;
use solana_program_test::*;

mod program_test;

use program_test::*;
use spl_governance::{
    error::GovernanceError,
    state::{enums::MintMaxVoteWeightSource, realm::RealmConfigArgs},
};

#[tokio::test]
async fn test_set_realm_config() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let mut realm_cookie = governance_test.with_realm().await;

    let config_args = RealmConfigArgs {
        use_council_mint: true,

        community_mint_max_vote_weight_source: MintMaxVoteWeightSource::SupplyFraction(100),
        min_community_tokens_to_create_governance: 10,
        use_community_voter_weight_addin: false,
    };

    // Act

    governance_test
        .set_realm_config(&mut realm_cookie, &config_args)
        .await
        .unwrap();

    // Assert
    let realm_account = governance_test
        .get_realm_account(&realm_cookie.address)
        .await;

    assert_eq!(realm_cookie.account, realm_account);
}

#[tokio::test]
async fn test_set_realm_config_with_authority_must_sign_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let mut realm_cookie = governance_test.with_realm().await;

    let config_args = RealmConfigArgs {
        use_council_mint: true,

        community_mint_max_vote_weight_source: MintMaxVoteWeightSource::SupplyFraction(100),
        min_community_tokens_to_create_governance: 10,
        use_community_voter_weight_addin: false,
    };

    // Act

    let err = governance_test
        .set_realm_config_using_instruction(
            &mut realm_cookie,
            &config_args,
            |i| i.accounts[1].is_signer = false,
            Some(&[]),
        )
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(err, GovernanceError::RealmAuthorityMustSign.into());
}

#[tokio::test]
async fn test_set_realm_config_with_no_authority_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let mut realm_cookie = governance_test.with_realm().await;

    let config_args = RealmConfigArgs {
        use_council_mint: true,

        community_mint_max_vote_weight_source: MintMaxVoteWeightSource::SupplyFraction(100),
        min_community_tokens_to_create_governance: 10,
        use_community_voter_weight_addin: false,
    };

    governance_test
        .set_realm_authority(&realm_cookie, &None)
        .await
        .unwrap();

    // Act

    let err = governance_test
        .set_realm_config_using_instruction(
            &mut realm_cookie,
            &config_args,
            |i| i.accounts[1].is_signer = false,
            Some(&[]),
        )
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(err, GovernanceError::RealmHasNoAuthority.into());
}

#[tokio::test]
async fn test_set_realm_config_with_invalid_authority_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let mut realm_cookie = governance_test.with_realm().await;

    let config_args = RealmConfigArgs {
        use_council_mint: true,

        community_mint_max_vote_weight_source: MintMaxVoteWeightSource::SupplyFraction(100),
        min_community_tokens_to_create_governance: 10,
        use_community_voter_weight_addin: false,
    };

    let realm_cookie2 = governance_test.with_realm().await;

    // Try to use authority from other realm
    realm_cookie.realm_authority = realm_cookie2.realm_authority;

    // Act

    let err = governance_test
        .set_realm_config(&mut realm_cookie, &config_args)
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(err, GovernanceError::InvalidAuthorityForRealm.into());
}

#[tokio::test]
async fn test_set_realm_config_with_remove_council() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let mut realm_cookie = governance_test.with_realm().await;

    let config_args = RealmConfigArgs {
        use_council_mint: false,

        community_mint_max_vote_weight_source: MintMaxVoteWeightSource::SupplyFraction(100),
        min_community_tokens_to_create_governance: 10,
        use_community_voter_weight_addin: false,
    };

    // Act
    governance_test
        .set_realm_config(&mut realm_cookie, &config_args)
        .await
        .unwrap();

    // Assert
    let realm_account = governance_test
        .get_realm_account(&realm_cookie.address)
        .await;

    assert_eq!(realm_cookie.account, realm_account);
    assert_eq!(None, realm_account.config.council_mint);
}

#[tokio::test]
async fn test_set_realm_config_with_council_change_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let mut realm_cookie = governance_test.with_realm().await;

    let config_args = RealmConfigArgs {
        use_council_mint: true,

        community_mint_max_vote_weight_source: MintMaxVoteWeightSource::SupplyFraction(100),
        min_community_tokens_to_create_governance: 10,
        use_community_voter_weight_addin: false,
    };

    // Try to replace council mint
    realm_cookie.account.config.council_mint = serde::__private::Some(Pubkey::new_unique());

    // Act
    let err = governance_test
        .set_realm_config(&mut realm_cookie, &config_args)
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(
        err,
        GovernanceError::RealmCouncilMintChangeIsNotSupported.into()
    );
}

#[tokio::test]
async fn test_set_realm_config_with_council_restore_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let mut realm_cookie = governance_test.with_realm().await;

    let mut config_args = RealmConfigArgs {
        use_council_mint: false,

        community_mint_max_vote_weight_source: MintMaxVoteWeightSource::SupplyFraction(100),
        min_community_tokens_to_create_governance: 10,
        use_community_voter_weight_addin: false,
    };

    governance_test
        .set_realm_config(&mut realm_cookie, &config_args)
        .await
        .unwrap();

    // Try to restore council mint after removing it
    config_args.use_council_mint = true;
    realm_cookie.account.config.council_mint = serde::__private::Some(Pubkey::new_unique());

    // Act
    let err = governance_test
        .set_realm_config(&mut realm_cookie, &config_args)
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(
        err,
        GovernanceError::RealmCouncilMintChangeIsNotSupported.into()
    );
}
