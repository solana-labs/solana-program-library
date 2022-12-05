#![cfg(feature = "test-sbf")]

use solana_program_test::*;

mod program_test;

use program_test::legacy::*;
use program_test::*;

use spl_governance::state::{
    enums::{VoteThreshold, VoteTipping},
    governance::DEFAULT_DEPOSIT_EXEMPT_PROPOSAL_COUNT,
};

#[tokio::test]
async fn test_create_proposal_and_migrate_governance_v1_to_v2() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let mut governance_cookie = governance_test
        .with_governance(
            &realm_cookie,
            &governed_account_cookie,
            &token_owner_record_cookie,
        )
        .await
        .unwrap();

    // Override Governance account with LegacyV1 version
    let mut governance_v1: LegacyGovernanceV1 = governance_cookie.account.clone().into();
    governance_v1.config.vote_threshold_percentage = VoteThresholdPercentage::YesVote(55);

    governance_test.set_account(&governance_cookie.address, &governance_v1);

    // Act
    governance_test
        .with_proposal(&token_owner_record_cookie, &mut governance_cookie)
        .await
        .unwrap();

    // Assert
    let governance_account = governance_test
        .get_governance_account(&governance_cookie.address)
        .await;

    assert_eq!(1, governance_account.active_proposal_count);

    assert_eq!(
        VoteThreshold::YesVotePercentage(55),
        governance_account.config.council_vote_threshold
    );

    assert_eq!(
        VoteThreshold::YesVotePercentage(55),
        governance_account.config.council_veto_vote_threshold
    );

    assert_eq!(
        VoteThreshold::Disabled,
        governance_account.config.community_veto_vote_threshold
    );

    assert_eq!(
        DEFAULT_DEPOSIT_EXEMPT_PROPOSAL_COUNT,
        governance_account.config.deposit_exempt_proposal_count
    );

    assert_eq!(0, governance_account.reserved1);

    assert_eq!(
        VoteTipping::Strict,
        governance_account.config.council_vote_tipping
    );

    assert_eq!(
        VoteTipping::Strict,
        governance_account.config.community_vote_tipping
    );
}
