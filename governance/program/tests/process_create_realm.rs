#![cfg(feature = "test-sbf")]

use solana_program_test::*;

mod program_test;

use program_test::*;
use spl_governance::state::{enums::MintMaxVoterWeightSource, realm::get_realm_address};

use crate::program_test::args::RealmSetupArgs;

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

    let realm_setup_args = RealmSetupArgs {
        use_council_mint: false,
        community_mint_max_voter_weight_source: MintMaxVoterWeightSource::SupplyFraction(1),
        min_community_weight_to_create_governance: 1,
        ..Default::default()
    };

    // Act
    let realm_cookie = governance_test
        .with_realm_using_args(&realm_setup_args)
        .await;

    // Assert
    let realm_account = governance_test
        .get_realm_account(&realm_cookie.address)
        .await;

    assert_eq!(realm_cookie.account, realm_account);
}

#[tokio::test]
async fn test_create_realm_with_max_voter_weight_absolute_value() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_setup_args = RealmSetupArgs {
        community_mint_max_voter_weight_source: MintMaxVoterWeightSource::Absolute(1),
        ..Default::default()
    };

    // Act
    let realm_cookie = governance_test
        .with_realm_using_args(&realm_setup_args)
        .await;

    // Assert
    let realm_account = governance_test
        .get_realm_account(&realm_cookie.address)
        .await;

    assert_eq!(realm_cookie.account, realm_account);
    assert_eq!(
        realm_cookie
            .account
            .config
            .community_mint_max_voter_weight_source,
        MintMaxVoterWeightSource::Absolute(1)
    );
}

#[tokio::test]
async fn test_create_realm_for_existing_pda() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_name = format!("Realm #{}", governance_test.next_realm_id).to_string();
    let realm_address = get_realm_address(&governance_test.program_id, &realm_name);

    let rent_exempt = governance_test.bench.rent.minimum_balance(0);

    governance_test
        .bench
        .transfer_sol(&realm_address, rent_exempt)
        .await;

    // Act
    let realm_cookie = governance_test.with_realm().await;

    // Assert
    let realm_account = governance_test
        .get_realm_account(&realm_cookie.address)
        .await;

    assert_eq!(realm_cookie.account, realm_account);
}
