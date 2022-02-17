#![cfg(feature = "test-bpf")]

use solana_program_test::*;

mod program_test;

use program_test::*;
use spl_governance::state::{
    enums::MintMaxVoteWeightSource,
    realm::{get_realm_address, RealmConfigArgs},
};

use self::args::SetRealmConfigArgs;

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

    let realm_config_args = RealmConfigArgs {
        use_council_mint: false,
        community_mint_max_vote_weight_source: MintMaxVoteWeightSource::SupplyFraction(1),
        min_community_weight_to_create_governance: 10,
        use_community_voter_weight_addin: false,
        use_max_community_voter_weight_addin: false,
    };

    let set_realm_config_args = SetRealmConfigArgs {
        realm_config_args,
        community_voter_weight_addin: None,
        max_community_voter_weight_addin: None,
    };

    // Act
    let realm_cookie = governance_test
        .with_realm_using_config_args(&set_realm_config_args)
        .await;

    // Assert
    let realm_account = governance_test
        .get_realm_account(&realm_cookie.address)
        .await;

    assert_eq!(realm_cookie.account, realm_account);
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
