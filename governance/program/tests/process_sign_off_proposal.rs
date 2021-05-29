#![cfg(feature = "test-bpf")]

mod program_test;

use solana_program_test::tokio;

use program_test::*;
use spl_governance::state::enums::ProposalState;

#[tokio::test]
async fn test_sign_off_proposal() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    let mut account_governance_cookie = governance_test
        .with_account_governance(&realm_cookie, &governed_account_cookie)
        .await
        .unwrap();

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await;

    let proposal_cookie = governance_test
        .with_proposal(&token_owner_record_cookie, &mut account_governance_cookie)
        .await
        .unwrap();

    let signatory_record_cookie = governance_test
        .with_signatory(&proposal_cookie, &token_owner_record_cookie)
        .await
        .unwrap();

    // Act
    governance_test
        .sign_off_proposal(&proposal_cookie, &signatory_record_cookie)
        .await
        .unwrap();

    // Assert
    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    assert_eq!(1, proposal_account.signatories_count);
    assert_eq!(1, proposal_account.signatories_signed_off_count);
    assert_eq!(ProposalState::Voting, proposal_account.state);
    assert_eq!(Some(1), proposal_account.signing_off_at);
    assert_eq!(Some(1), proposal_account.voting_at);

    let signatory_record_account = governance_test
        .get_signatory_record_account(&signatory_record_cookie.address)
        .await;

    assert_eq!(true, signatory_record_account.signed_off);
}
