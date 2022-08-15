#![cfg(feature = "test-bpf")]

use solana_program::pubkey::Pubkey;
use solana_program_test::*;

mod program_test;

use program_test::*;
use spl_governance::{
    error::GovernanceError,
    state::{
        enums::MintMaxVoteWeightSource,
        realm::{GoverningTokenConfigArgs, RealmConfigArgs},
        realm_config::GoverningTokenType,
    },
};

use self::args::SetRealmConfigArgs;

#[tokio::test]
async fn test_set_realm_config() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let mut realm_cookie = governance_test.with_realm().await;

    let realm_config_args = RealmConfigArgs {
        use_council_mint: true,

        community_mint_max_vote_weight_source: MintMaxVoteWeightSource::SupplyFraction(100),
        min_community_weight_to_create_governance: 10,
        community_token_config_args: GoverningTokenConfigArgs::default(),
        council_token_config_args: GoverningTokenConfigArgs::default(),
    };

    let set_realm_config_args = SetRealmConfigArgs {
        realm_config_args,
        community_voter_weight_addin: None,
        max_community_voter_weight_addin: None,
    };

    // Act

    governance_test
        .set_realm_config(&mut realm_cookie, &set_realm_config_args)
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

    let realm_config_args = RealmConfigArgs {
        use_council_mint: true,

        community_mint_max_vote_weight_source: MintMaxVoteWeightSource::SupplyFraction(100),
        min_community_weight_to_create_governance: 10,
        community_token_config_args: GoverningTokenConfigArgs::default(),
        council_token_config_args: GoverningTokenConfigArgs::default(),
    };

    let set_realm_config_args = SetRealmConfigArgs {
        realm_config_args,
        community_voter_weight_addin: None,
        max_community_voter_weight_addin: None,
    };

    // Act

    let err = governance_test
        .set_realm_config_using_instruction(
            &mut realm_cookie,
            &set_realm_config_args,
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

    let realm_config_args = RealmConfigArgs {
        use_council_mint: true,

        community_mint_max_vote_weight_source: MintMaxVoteWeightSource::SupplyFraction(100),
        min_community_weight_to_create_governance: 10,
        community_token_config_args: GoverningTokenConfigArgs::default(),
        council_token_config_args: GoverningTokenConfigArgs::default(),
    };

    let set_realm_config_args = SetRealmConfigArgs {
        realm_config_args,
        community_voter_weight_addin: None,
        max_community_voter_weight_addin: None,
    };

    governance_test
        .set_realm_authority(&realm_cookie, None)
        .await
        .unwrap();

    // Act

    let err = governance_test
        .set_realm_config_using_instruction(
            &mut realm_cookie,
            &set_realm_config_args,
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

    let realm_config_args = RealmConfigArgs {
        use_council_mint: true,

        community_mint_max_vote_weight_source: MintMaxVoteWeightSource::SupplyFraction(100),
        min_community_weight_to_create_governance: 10,
        community_token_config_args: GoverningTokenConfigArgs::default(),
        council_token_config_args: GoverningTokenConfigArgs::default(),
    };

    let set_realm_config_args = SetRealmConfigArgs {
        realm_config_args,
        community_voter_weight_addin: None,
        max_community_voter_weight_addin: None,
    };

    let realm_cookie2 = governance_test.with_realm().await;

    // Try to use authority from other realm
    realm_cookie.realm_authority = realm_cookie2.realm_authority;

    // Act

    let err = governance_test
        .set_realm_config(&mut realm_cookie, &set_realm_config_args)
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

    let realm_config_args = RealmConfigArgs {
        use_council_mint: false,

        community_mint_max_vote_weight_source: MintMaxVoteWeightSource::SupplyFraction(100),
        min_community_weight_to_create_governance: 10,
        community_token_config_args: GoverningTokenConfigArgs::default(),
        council_token_config_args: GoverningTokenConfigArgs::default(),
    };

    let set_realm_config_args = SetRealmConfigArgs {
        realm_config_args,
        community_voter_weight_addin: None,
        max_community_voter_weight_addin: None,
    };

    // Act
    governance_test
        .set_realm_config(&mut realm_cookie, &set_realm_config_args)
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

    let realm_config_args = RealmConfigArgs {
        use_council_mint: true,

        community_mint_max_vote_weight_source: MintMaxVoteWeightSource::SupplyFraction(100),
        min_community_weight_to_create_governance: 10,
        community_token_config_args: GoverningTokenConfigArgs::default(),
        council_token_config_args: GoverningTokenConfigArgs::default(),
    };

    let set_realm_config_args = SetRealmConfigArgs {
        realm_config_args,
        community_voter_weight_addin: None,
        max_community_voter_weight_addin: None,
    };

    // Try to replace council mint
    realm_cookie.account.config.council_mint = serde::__private::Some(Pubkey::new_unique());

    // Act
    let err = governance_test
        .set_realm_config(&mut realm_cookie, &set_realm_config_args)
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

    let realm_config_args = RealmConfigArgs {
        use_council_mint: false,

        community_mint_max_vote_weight_source: MintMaxVoteWeightSource::SupplyFraction(100),
        min_community_weight_to_create_governance: 10,
        community_token_config_args: GoverningTokenConfigArgs::default(),
        council_token_config_args: GoverningTokenConfigArgs::default(),
    };

    let mut set_realm_config_args = SetRealmConfigArgs {
        realm_config_args,
        community_voter_weight_addin: None,
        max_community_voter_weight_addin: None,
    };

    governance_test
        .set_realm_config(&mut realm_cookie, &set_realm_config_args)
        .await
        .unwrap();

    // Try to restore council mint after removing it
    set_realm_config_args.realm_config_args.use_council_mint = true;
    realm_cookie.account.config.council_mint = serde::__private::Some(Pubkey::new_unique());

    // Act
    let err = governance_test
        .set_realm_config(&mut realm_cookie, &set_realm_config_args)
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
async fn test_set_realm_config_with_liquid_community_token_cannot_be_changed_to_memebership_error()
{
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let mut realm_cookie = governance_test.with_realm().await;

    let mut set_realm_config_args = SetRealmConfigArgs::default();

    // Try to change Community token type to Membership
    set_realm_config_args
        .realm_config_args
        .community_token_config_args
        .token_type = GoverningTokenType::Membership;

    // Act
    let err = governance_test
        .set_realm_config(&mut realm_cookie, &set_realm_config_args)
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(
        err,
        GovernanceError::CannotChangeCommunityTokenTypeToMemebership.into()
    );
}

#[tokio::test]
async fn test_set_realm_config_for_community_token() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let mut realm_cookie = governance_test.with_realm().await;

    let mut set_realm_config_args = SetRealmConfigArgs::default();

    // Change Community token type to Dormant and set plugins
    set_realm_config_args
        .realm_config_args
        .community_token_config_args = GoverningTokenConfigArgs {
        use_voter_weight_addin: true,
        use_max_voter_weight_addin: true,
        token_type: GoverningTokenType::Dormant,
    };

    set_realm_config_args.community_voter_weight_addin = Some(Pubkey::new_unique());
    set_realm_config_args.max_community_voter_weight_addin = Some(Pubkey::new_unique());

    // Act

    governance_test
        .set_realm_config(&mut realm_cookie, &set_realm_config_args)
        .await
        .unwrap();

    // Assert

    let realm_config_account = governance_test
        .get_realm_config_account(&realm_cookie.realm_config.address)
        .await;

    assert_eq!(
        realm_config_account.community_token_config.token_type,
        GoverningTokenType::Dormant
    );

    assert_eq!(
        realm_config_account
            .community_token_config
            .voter_weight_addin,
        set_realm_config_args.community_voter_weight_addin
    );

    assert_eq!(
        realm_config_account
            .community_token_config
            .max_voter_weight_addin,
        set_realm_config_args.max_community_voter_weight_addin
    );
}
