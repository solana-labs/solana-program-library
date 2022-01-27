#![cfg(feature = "test-bpf")]

use solana_program_test::*;

mod program_test;

use program_test::*;

#[tokio::test]
async fn test_cast_vote_with_max_voter_weight_addin() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_with_max_voter_weight_addin().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    let realm_cookie = governance_test.with_realm().await;

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    governance_test
        .with_max_voter_weight_addin_record(&token_owner_record_cookie)
        .await
        .unwrap();

    let mut account_governance_cookie = governance_test
        .with_account_governance(
            &realm_cookie,
            &governed_account_cookie,
            &token_owner_record_cookie,
        )
        .await
        .unwrap();

    let proposal_cookie = governance_test
        .with_signed_off_proposal(&token_owner_record_cookie, &mut account_governance_cookie)
        .await
        .unwrap();

    // // Act
    let _vote_record_cookie = governance_test
        .with_cast_vote(&proposal_cookie, &token_owner_record_cookie, YesNoVote::Yes)
        .await
        .unwrap();

    // // Assert

    // let vote_record_account = governance_test
    //     .get_vote_record_account(&vote_record_cookie.address)
    //     .await;

    // assert_eq!(120, vote_record_account.voter_weight);
    // assert_eq!(
    //     Vote::Approve(vec![VoteChoice {
    //         rank: 0,
    //         weight_percentage: 100
    //     }]),
    //     vote_record_account.vote
    // );

    // let proposal_account = governance_test
    //     .get_proposal_account(&proposal_cookie.address)
    //     .await;

    // assert_eq!(120, proposal_account.options[0].vote_weight);
}
