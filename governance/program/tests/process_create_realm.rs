#![cfg(feature = "test-bpf")]

use solana_program_test::*;

mod program_test;

use program_test::*;
use spl_governance::state::{enums::MintMaxVoteWeightSource, realm::RealmConfigArgs};

#[tokio::test]
async fn test_create_realm() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    // Act
    let realm_cookie = governance_test.with_realm().await;

    // Assert
    let realm_account = governance_test
        .get_realm_account(&realm_cookie.address)
        .await;

    assert_eq!(realm_cookie.account, realm_account);
}

#[tokio::test]
async fn test_create_realm_with_non_default_config() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let config_args = RealmConfigArgs {
        use_council_mint: false,

        community_mint_max_vote_weight_source: MintMaxVoteWeightSource::SupplyFraction(1),
        min_community_tokens_to_create_governance: 10,
    };

    // Act
    let realm_cookie = governance_test
        .with_realm_using_config_args(&config_args)
        .await;

    // Assert
    let realm_account = governance_test
        .get_realm_account(&realm_cookie.address)
        .await;

    assert_eq!(realm_cookie.account, realm_account);
}
