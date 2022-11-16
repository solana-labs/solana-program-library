#![cfg(feature = "test-sbf")]

mod program_test;

use solana_program_test::tokio;

use program_test::*;
use spl_governance::state::enums::ProposalState;

#[tokio::test]
async fn test_execute_complete_proposal_without_transaction() {
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

    let mut proposal_cookie = governance_test
        .with_proposal(&token_owner_record_cookie, &mut governance_cookie)
        .await
        .unwrap();

    let signatory_record_cookie = governance_test
        .with_signatory(&proposal_cookie, &token_owner_record_cookie)
        .await
        .unwrap();

    governance_test
        .sign_off_proposal(&proposal_cookie, &signatory_record_cookie)
        .await
        .unwrap();

    governance_test
        .with_cast_yes_no_vote(&proposal_cookie, &token_owner_record_cookie, YesNoVote::Yes)
        .await
        .unwrap();

    let clock = governance_test.bench.get_clock().await;

    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;
    assert_eq!(ProposalState::Succeeded, proposal_account.state);

    // Act
    governance_test
        .with_complete_proposal(&mut governance_cookie, &mut proposal_cookie)
        .await
        .unwrap();

    // Assert
    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    let yes_option = proposal_account.options.first().unwrap();
    assert_eq!(0, yes_option.transactions_count);
    assert_eq!(ProposalState::Completed, proposal_account.state);
    assert_eq!(Some(clock.unix_timestamp), proposal_account.closed_at);
}
