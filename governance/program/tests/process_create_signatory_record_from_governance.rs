#![cfg(feature = "test-bpf")]

mod program_test;

use solana_program_test::tokio;

use program_test::*;
use solana_program::pubkey::Pubkey;

#[tokio::test]
async fn test_create_signatory_record_from_governance() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_account_cookie = governance_test.with_governed_account().await;
    let signatory = Pubkey::new_unique();

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

    let proposal_transaction_cookie = governance_test
        .with_governance_required_signatory_transaction(
            &mut proposal_cookie,
            &token_owner_record_cookie,
            &governance_cookie,
            &signatory,
        )
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

    // Advance timestamp past hold_up_time
    governance_test
        .advance_clock_by_min_timespan(proposal_transaction_cookie.account.hold_up_time as u64)
        .await;

    governance_test
        .execute_proposal_transaction(&proposal_cookie, &proposal_transaction_cookie)
        .await
        .unwrap();

    let new_proposal_cookie = governance_test
        .with_proposal(&token_owner_record_cookie, &mut governance_cookie)
        .await
        .unwrap();

    // Act
    let new_signatory_record_cookie = governance_test
        .with_signatory_record_from_governance(&new_proposal_cookie, &governance_cookie, &signatory)
        .await
        .unwrap();

    // Assert
    let signatory_account = governance_test
        .get_signatory_record_account(&new_signatory_record_cookie.address)
        .await;

    assert_eq!(signatory_account.signatory, signatory);
    assert_eq!(signatory_account.proposal, new_proposal_cookie.address);
    assert_eq!(signatory_account.signed_off, false);

    let new_proposal_account = governance_test
        .get_proposal_account(&new_proposal_cookie.address)
        .await;

    assert_eq!(new_proposal_account.signatories_count, 1);
}
