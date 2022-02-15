#![cfg(feature = "test-bpf")]

mod program_test;

use solana_program_test::tokio;

use program_test::*;
use spl_governance::{error::GovernanceError, state::enums::ProposalState};

#[tokio::test]
async fn test_sign_off_proposal() {
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

    let proposal_cookie = governance_test
        .with_proposal(&token_owner_record_cookie, &mut governance_cookie)
        .await
        .unwrap();

    let signatory_record_cookie = governance_test
        .with_signatory(&proposal_cookie, &token_owner_record_cookie)
        .await
        .unwrap();

    let clock = governance_test.bench.get_clock().await;

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
    assert_eq!(Some(clock.unix_timestamp), proposal_account.signing_off_at);
    assert_eq!(Some(clock.unix_timestamp), proposal_account.voting_at);
    assert_eq!(Some(clock.slot), proposal_account.voting_at_slot);

    let signatory_record_account = governance_test
        .get_signatory_record_account(&signatory_record_cookie.address)
        .await;

    assert_eq!(true, signatory_record_account.signed_off);

    let realm_account = governance_test
        .get_realm_account(&realm_cookie.address)
        .await;

    assert_eq!(1, realm_account.voting_proposal_count);

    let governance_account = governance_test
        .get_governance_account(&governance_cookie.address)
        .await;

    assert_eq!(1, governance_account.voting_proposal_count);
}

#[tokio::test]
async fn test_sign_off_proposal_with_signatory_must_sign_error() {
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

    let proposal_cookie = governance_test
        .with_proposal(&token_owner_record_cookie, &mut governance_cookie)
        .await
        .unwrap();

    let signatory_record_cookie = governance_test
        .with_signatory(&proposal_cookie, &token_owner_record_cookie)
        .await
        .unwrap();

    // Act
    let err = governance_test
        .sign_off_proposal_using_instruction(
            &proposal_cookie,
            &signatory_record_cookie,
            |i| i.accounts[3].is_signer = false, // signatory
            Some(&[]),
        )
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(err, GovernanceError::SignatoryMustSign.into());
}

#[tokio::test]
async fn test_sign_off_proposal_by_owner() {
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

    let proposal_cookie = governance_test
        .with_proposal(&token_owner_record_cookie, &mut governance_cookie)
        .await
        .unwrap();

    let clock = governance_test.bench.get_clock().await;

    // Act
    governance_test
        .sign_off_proposal_by_owner(&proposal_cookie, &token_owner_record_cookie)
        .await
        .unwrap();

    // Assert
    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    assert_eq!(0, proposal_account.signatories_count);
    assert_eq!(0, proposal_account.signatories_signed_off_count);
    assert_eq!(ProposalState::Voting, proposal_account.state);
    assert_eq!(Some(clock.unix_timestamp), proposal_account.signing_off_at);
    assert_eq!(Some(clock.unix_timestamp), proposal_account.voting_at);
    assert_eq!(Some(clock.slot), proposal_account.voting_at_slot);
}

#[tokio::test]
async fn test_sign_off_proposal_by_owner_with_owner_must_sign_error() {
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

    let proposal_cookie = governance_test
        .with_proposal(&token_owner_record_cookie, &mut governance_cookie)
        .await
        .unwrap();

    // Act

    let err = governance_test
        .sign_off_proposal_by_owner_using_instruction(
            &proposal_cookie,
            &token_owner_record_cookie,
            |i| i.accounts[3].is_signer = false, // signatory
            Some(&[]),
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
async fn test_sign_off_proposal_by_owner_with_other_proposal_owner_error() {
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

    let proposal_cookie = governance_test
        .with_proposal(&token_owner_record_cookie, &mut governance_cookie)
        .await
        .unwrap();

    let token_owner_record_cookie2 = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    // Act

    let err = governance_test
        .sign_off_proposal_by_owner(&proposal_cookie, &token_owner_record_cookie2)
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(err, GovernanceError::InvalidProposalOwnerAccount.into());
}

#[tokio::test]
async fn test_sign_off_proposal_by_owner_with_existing_signatories_error() {
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

    let proposal_cookie = governance_test
        .with_proposal(&token_owner_record_cookie, &mut governance_cookie)
        .await
        .unwrap();

    governance_test
        .with_signatory(&proposal_cookie, &token_owner_record_cookie)
        .await
        .unwrap();

    // Act

    let err = governance_test
        .sign_off_proposal_by_owner(&proposal_cookie, &token_owner_record_cookie)
        .await
        .err()
        .unwrap();

    // Assert

    assert_eq!(err, GovernanceError::InvalidSignatoryAddress.into());
}
