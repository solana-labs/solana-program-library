#![cfg(feature = "test-bpf")]

use solana_program_test::*;

mod program_test;

use program_test::*;
use spl_governance::state::{enums::MintMaxVoteWeightSource, realm::RealmConfigArgs};

#[tokio::test]
async fn test_set_realm_config() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let mut realm_cookie = governance_test.with_realm().await;

    let config_args = RealmConfigArgs {
        use_council_mint: true,
        use_custodian: true,
        community_mint_max_vote_weight_source: MintMaxVoteWeightSource::SupplyFraction(100),
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
