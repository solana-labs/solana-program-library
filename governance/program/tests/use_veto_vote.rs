#![cfg(feature = "test-bpf")]

mod program_test;

use solana_program_test::tokio;

use program_test::*;
use spl_governance::state::{enums::ProposalState, vote_record::Vote};

#[tokio::test]
async fn test_cast_veto_vote() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    let token_owner_record_cookie = governance_test
        .with_council_token_deposit(&realm_cookie)
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

    let proposal_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let proposal_cookie = governance_test
        .with_signed_off_proposal(&proposal_owner_record_cookie, &mut governance_cookie)
        .await
        .unwrap();

    let clock = governance_test.bench.get_clock().await;

    // Act
    let vote_record_cookie = governance_test
        .with_cast_vote(&proposal_cookie, &token_owner_record_cookie, Vote::Veto)
        .await
        .unwrap();

    // Assert
    let vote_record_account = governance_test
        .get_vote_record_account(&vote_record_cookie.address)
        .await;

    assert_eq!(vote_record_cookie.account, vote_record_account);

    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    assert_eq!(
        token_owner_record_cookie
            .account
            .governing_token_deposit_amount,
        proposal_account.veto_vote_weight
    );

    assert_eq!(proposal_account.state, ProposalState::Vetoed);
    assert_eq!(
        proposal_account.voting_completed_at,
        Some(clock.unix_timestamp)
    );

    assert_eq!(Some(100), proposal_account.max_vote_weight);
    assert_eq!(
        Some(governance_cookie.account.config.council_veto_vote_threshold),
        proposal_account.vote_threshold
    );

    let token_owner_record = governance_test
        .get_token_owner_record_account(&token_owner_record_cookie.address)
        .await;

    assert_eq!(1, token_owner_record.unrelinquished_votes_count);
    assert_eq!(1, token_owner_record.total_votes_count);

    let realm_account = governance_test
        .get_realm_account(&realm_cookie.address)
        .await;

    assert_eq!(0, realm_account.voting_proposal_count);

    let governance_account = governance_test
        .get_governance_account(&governance_cookie.address)
        .await;

    assert_eq!(0, governance_account.voting_proposal_count);
}
