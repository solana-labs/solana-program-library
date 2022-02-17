#![cfg(feature = "test-bpf")]

use solana_program_test::*;

mod program_test;

use program_test::*;
use spl_governance::state::enums::ProposalState;

#[tokio::test]
async fn test_cast_vote_with_all_addin() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_with_all_addins().await;

    let realm_cookie = governance_test.with_realm().await;

    let mut token_owner_record_cookie =
        governance_test.with_token_owner_record(&realm_cookie).await;

    // voter weight 120
    governance_test
        .with_voter_weight_addin_record(&mut token_owner_record_cookie)
        .await
        .unwrap();

    // max voter weight 250
    governance_test
        .with_max_voter_weight_addin_record_impl(&mut token_owner_record_cookie, 250, None)
        .await
        .unwrap();

    let governed_account_cookie = governance_test.with_governed_account().await;

    let mut governance_cookie = governance_test
        .with_governance(
            &realm_cookie,
            &governed_account_cookie,
            &token_owner_record_cookie,
        )
        .await
        .unwrap();

    let proposal_cookie = governance_test
        .with_signed_off_proposal(&token_owner_record_cookie, &mut governance_cookie)
        .await
        .unwrap();

    // Act

    let vote_record_cookie = governance_test
        .with_cast_vote(&proposal_cookie, &token_owner_record_cookie, YesNoVote::Yes)
        .await
        .unwrap();

    // Assert
    let vote_record_account = governance_test
        .get_vote_record_account(&vote_record_cookie.address)
        .await;

    assert_eq!(120, vote_record_account.voter_weight);

    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    assert_eq!(proposal_account.state, ProposalState::Voting)
}

#[tokio::test]
async fn test_tip_vote_with_all_addin() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_with_all_addins().await;

    let realm_cookie = governance_test.with_realm().await;

    let mut token_owner_record_cookie =
        governance_test.with_token_owner_record(&realm_cookie).await;

    // voter weight 120
    governance_test
        .with_voter_weight_addin_record(&mut token_owner_record_cookie)
        .await
        .unwrap();

    // max voter weight 200
    governance_test
        .with_max_voter_weight_addin_record(&mut token_owner_record_cookie)
        .await
        .unwrap();

    let governed_account_cookie = governance_test.with_governed_account().await;

    let mut governance_cookie = governance_test
        .with_governance(
            &realm_cookie,
            &governed_account_cookie,
            &token_owner_record_cookie,
        )
        .await
        .unwrap();

    let proposal_cookie = governance_test
        .with_signed_off_proposal(&token_owner_record_cookie, &mut governance_cookie)
        .await
        .unwrap();

    // Act

    let vote_record_cookie = governance_test
        .with_cast_vote(&proposal_cookie, &token_owner_record_cookie, YesNoVote::No)
        .await
        .unwrap();

    // Assert
    let vote_record_account = governance_test
        .get_vote_record_account(&vote_record_cookie.address)
        .await;

    assert_eq!(120, vote_record_account.voter_weight);

    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    assert_eq!(proposal_account.state, ProposalState::Defeated)
}

#[tokio::test]
async fn test_finalize_vote_with_all_addin() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_with_all_addins().await;

    let realm_cookie = governance_test.with_realm().await;

    let mut token_owner_record_cookie =
        governance_test.with_token_owner_record(&realm_cookie).await;

    // voter weight 120
    governance_test
        .with_voter_weight_addin_record(&mut token_owner_record_cookie)
        .await
        .unwrap();

    // max voter weight 400
    let max_voter_weight_record_cookie = governance_test
        .with_max_voter_weight_addin_record_impl(&mut token_owner_record_cookie, 400, None)
        .await
        .unwrap();

    let governed_account_cookie = governance_test.with_governed_account().await;

    let mut governance_cookie = governance_test
        .with_governance(
            &realm_cookie,
            &governed_account_cookie,
            &token_owner_record_cookie,
        )
        .await
        .unwrap();

    let proposal_cookie = governance_test
        .with_signed_off_proposal(&token_owner_record_cookie, &mut governance_cookie)
        .await
        .unwrap();

    let vote_record_cookie = governance_test
        .with_cast_vote(&proposal_cookie, &token_owner_record_cookie, YesNoVote::No)
        .await
        .unwrap();

    governance_test
        .advance_clock_past_voting_time(&governance_cookie)
        .await;

    // Act

    governance_test
        .finalize_vote(
            &realm_cookie,
            &proposal_cookie,
            Some(max_voter_weight_record_cookie),
        )
        .await
        .unwrap();

    // Assert
    let vote_record_account = governance_test
        .get_vote_record_account(&vote_record_cookie.address)
        .await;

    assert_eq!(120, vote_record_account.voter_weight);

    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    assert_eq!(proposal_account.state, ProposalState::Defeated)
}
