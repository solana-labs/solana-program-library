#![cfg(feature = "test-bpf")]

mod program_test;

use solana_program_test::tokio;

use program_test::*;
use spl_governance::error::GovernanceError;

#[tokio::test]
async fn test_remove_transaction() {
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

    let proposal_transaction_cookie = governance_test
        .with_nop_transaction(&mut proposal_cookie, &token_owner_record_cookie, 0, None)
        .await
        .unwrap();

    // Act

    governance_test
        .remove_transaction(
            &mut proposal_cookie,
            &token_owner_record_cookie,
            &proposal_transaction_cookie,
        )
        .await
        .unwrap();

    // Assert

    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    let yes_option = proposal_account.options.first().unwrap();

    assert_eq!(yes_option.transactions_count, 0);
    assert_eq!(yes_option.transactions_next_index, 1);
    assert_eq!(yes_option.transactions_executed_count, 0);

    let proposal_transaction_account = governance_test
        .bench
        .get_account(&proposal_transaction_cookie.address)
        .await;

    assert_eq!(None, proposal_transaction_account);
}

#[tokio::test]
async fn test_replace_transaction() {
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

    let proposal_transaction_cookie = governance_test
        .with_nop_transaction(&mut proposal_cookie, &token_owner_record_cookie, 0, None)
        .await
        .unwrap();

    governance_test
        .with_nop_transaction(&mut proposal_cookie, &token_owner_record_cookie, 0, None)
        .await
        .unwrap();

    // Act

    governance_test
        .remove_transaction(
            &mut proposal_cookie,
            &token_owner_record_cookie,
            &proposal_transaction_cookie,
        )
        .await
        .unwrap();

    let proposal_transaction_cookie2 = governance_test
        .with_nop_transaction(&mut proposal_cookie, &token_owner_record_cookie, 0, Some(0))
        .await
        .unwrap();

    // Assert
    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    let yes_option = proposal_account.options.first().unwrap();

    assert_eq!(yes_option.transactions_count, 2);
    assert_eq!(yes_option.transactions_next_index, 2);

    let proposal_transaction_account2 = governance_test
        .get_proposal_transaction_account(&proposal_transaction_cookie2.address)
        .await;

    assert_eq!(
        proposal_transaction_cookie2.account,
        proposal_transaction_account2
    );
}

#[tokio::test]
async fn test_remove_front_transaction() {
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

    let proposal_transaction_cookie = governance_test
        .with_nop_transaction(&mut proposal_cookie, &token_owner_record_cookie, 0, None)
        .await
        .unwrap();

    governance_test
        .with_nop_transaction(&mut proposal_cookie, &token_owner_record_cookie, 0, None)
        .await
        .unwrap();

    // Act

    governance_test
        .remove_transaction(
            &mut proposal_cookie,
            &token_owner_record_cookie,
            &proposal_transaction_cookie,
        )
        .await
        .unwrap();

    // Assert
    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    let yes_option = proposal_account.options.first().unwrap();

    assert_eq!(yes_option.transactions_count, 1);
    assert_eq!(yes_option.transactions_next_index, 2);

    let proposal_transaction_account = governance_test
        .bench
        .get_account(&proposal_transaction_cookie.address)
        .await;

    assert_eq!(None, proposal_transaction_account);
}

#[tokio::test]
async fn test_remove_transaction_with_owner_or_delegate_must_sign_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    let mut token_owner_record_cookie = governance_test
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

    let proposal_transaction_cookie = governance_test
        .with_nop_transaction(&mut proposal_cookie, &token_owner_record_cookie, 0, None)
        .await
        .unwrap();

    let token_owner_record_cookie2 = governance_test
        .with_council_token_deposit(&realm_cookie)
        .await
        .unwrap();

    token_owner_record_cookie.token_owner = token_owner_record_cookie2.token_owner;

    // Act
    let err = governance_test
        .remove_transaction(
            &mut proposal_cookie,
            &token_owner_record_cookie,
            &proposal_transaction_cookie,
        )
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(
        err,
        GovernanceError::GoverningTokenOwnerOrDelegateMustSign.into()
    );
}

#[tokio::test]
async fn test_remove_transaction_with_proposal_not_editable_error() {
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

    let proposal_transaction_cookie = governance_test
        .with_nop_transaction(&mut proposal_cookie, &token_owner_record_cookie, 0, None)
        .await
        .unwrap();

    governance_test
        .cancel_proposal(&proposal_cookie, &token_owner_record_cookie)
        .await
        .unwrap();

    // Act
    let err = governance_test
        .remove_transaction(
            &mut proposal_cookie,
            &token_owner_record_cookie,
            &proposal_transaction_cookie,
        )
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(
        err,
        GovernanceError::InvalidStateCannotEditTransactions.into()
    );
}

#[tokio::test]
async fn test_remove_transaction_with_proposal_transaction_from_other_proposal_error() {
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

    governance_test
        .with_nop_transaction(&mut proposal_cookie, &token_owner_record_cookie, 0, None)
        .await
        .unwrap();

    let token_owner_record_cookie2 = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let mut proposal_cookie2 = governance_test
        .with_proposal(&token_owner_record_cookie2, &mut governance_cookie)
        .await
        .unwrap();

    let proposal_transaction_cookie2 = governance_test
        .with_nop_transaction(&mut proposal_cookie2, &token_owner_record_cookie2, 0, None)
        .await
        .unwrap();

    // Act
    let err = governance_test
        .remove_transaction(
            &mut proposal_cookie,
            &token_owner_record_cookie,
            &proposal_transaction_cookie2,
        )
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(
        err,
        GovernanceError::InvalidProposalForProposalTransaction.into()
    );
}
