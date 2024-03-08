#![cfg(feature = "test-sbf")]

mod program_test;

use {
    program_test::*,
    solana_program::program_error::ProgramError,
    solana_program_test::tokio,
    solana_sdk::{pubkey::Pubkey, signature::Signer},
    spl_governance::{
        error::GovernanceError,
        instruction::{add_signatory, AddSignatoryAuthority, GovernanceInstruction},
    },
};

#[tokio::test]
async fn test_add_signatory() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let mut governance_cookie = governance_test
        .with_governance(&realm_cookie, &token_owner_record_cookie)
        .await
        .unwrap();

    let proposal_cookie = governance_test
        .with_proposal(&token_owner_record_cookie, &mut governance_cookie)
        .await
        .unwrap();

    // Act
    let signatory_record_cookie = governance_test
        .with_signatory(
            &proposal_cookie,
            &governance_cookie,
            &token_owner_record_cookie,
        )
        .await
        .unwrap();

    // Assert
    let signatory_record_account = governance_test
        .get_signatory_record_account(&signatory_record_cookie.address)
        .await;

    assert_eq!(signatory_record_cookie.account, signatory_record_account);

    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    assert_eq!(1, proposal_account.signatories_count);
}

#[tokio::test]
async fn test_add_signatory_with_owner_or_delegate_must_sign_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;

    let mut token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let mut governance_cookie = governance_test
        .with_governance(&realm_cookie, &token_owner_record_cookie)
        .await
        .unwrap();

    let proposal_cookie = governance_test
        .with_proposal(&token_owner_record_cookie, &mut governance_cookie)
        .await
        .unwrap();

    let other_token_owner_record_cookie = governance_test
        .with_council_token_deposit(&realm_cookie)
        .await
        .unwrap();

    token_owner_record_cookie.token_owner = other_token_owner_record_cookie.token_owner;

    // Act
    let err = governance_test
        .with_signatory(
            &proposal_cookie,
            &governance_cookie,
            &token_owner_record_cookie,
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
async fn test_add_signatory_with_invalid_proposal_owner_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;

    let mut token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let mut governance_cookie = governance_test
        .with_governance(&realm_cookie, &token_owner_record_cookie)
        .await
        .unwrap();

    let proposal_cookie = governance_test
        .with_proposal(&token_owner_record_cookie, &mut governance_cookie)
        .await
        .unwrap();

    let other_token_owner_record_cookie = governance_test
        .with_council_token_deposit(&realm_cookie)
        .await
        .unwrap();

    token_owner_record_cookie.address = other_token_owner_record_cookie.address;

    // Act
    let err = governance_test
        .with_signatory(
            &proposal_cookie,
            &governance_cookie,
            &token_owner_record_cookie,
        )
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(err, GovernanceError::InvalidProposalOwnerAccount.into());
}

#[tokio::test]
async fn test_add_signatory_for_required_signatory() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;

    let signatory = Pubkey::new_unique();

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let mut governance_cookie = governance_test
        .with_governance(&realm_cookie, &token_owner_record_cookie)
        .await
        .unwrap();

    let mut proposal_cookie = governance_test
        .with_proposal(&token_owner_record_cookie, &mut governance_cookie)
        .await
        .unwrap();

    let signatory_record_cookie = governance_test
        .with_signatory(
            &proposal_cookie,
            &governance_cookie,
            &token_owner_record_cookie,
        )
        .await
        .unwrap();

    let proposal_transaction_cookie = governance_test
        .with_add_required_signatory_transaction(
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
        .advance_clock_by_min_timespan(
            governance_cookie.account.config.transactions_hold_up_time as u64,
        )
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
        .with_signatory_record_for_required_signatory(
            &new_proposal_cookie,
            &governance_cookie,
            &signatory,
        )
        .await
        .unwrap();

    // Assert
    let signatory_account = governance_test
        .get_signatory_record_account(&new_signatory_record_cookie.address)
        .await;

    assert_eq!(signatory_account.signatory, signatory);
    assert_eq!(signatory_account.proposal, new_proposal_cookie.address);
    assert!(!signatory_account.signed_off);

    let new_proposal_account = governance_test
        .get_proposal_account(&new_proposal_cookie.address)
        .await;

    assert_eq!(new_proposal_account.signatories_count, 1);
}

#[tokio::test]
async fn test_add_signatory_for_required_signatory_multiple_times_err() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;

    let signatory = Pubkey::new_unique();

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let mut governance_cookie = governance_test
        .with_governance(&realm_cookie, &token_owner_record_cookie)
        .await
        .unwrap();

    let mut proposal_cookie = governance_test
        .with_proposal(&token_owner_record_cookie, &mut governance_cookie)
        .await
        .unwrap();

    let signatory_record_cookie = governance_test
        .with_signatory(
            &proposal_cookie,
            &governance_cookie,
            &token_owner_record_cookie,
        )
        .await
        .unwrap();

    let proposal_transaction_cookie = governance_test
        .with_add_required_signatory_transaction(
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
        .advance_clock_by_min_timespan(
            governance_cookie.account.config.transactions_hold_up_time as u64,
        )
        .await;

    governance_test
        .execute_proposal_transaction(&proposal_cookie, &proposal_transaction_cookie)
        .await
        .unwrap();

    let new_proposal_cookie = governance_test
        .with_proposal(&token_owner_record_cookie, &mut governance_cookie)
        .await
        .unwrap();

    governance_test
        .with_signatory_record_for_required_signatory(
            &new_proposal_cookie,
            &governance_cookie,
            &signatory,
        )
        .await
        .unwrap();
    governance_test.advance_clock().await;

    // Act
    let err = governance_test
        .with_signatory_record_for_required_signatory(
            &new_proposal_cookie,
            &governance_cookie,
            &signatory,
        )
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(err, GovernanceError::SignatoryRecordAlreadyExists.into());
}

#[tokio::test]
pub async fn test_add_optional_signatory_before_all_required_signatories_err() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let (token_owner_record_cookie, mut governance_cookie, _, _) = governance_test
        .with_governance_with_required_signatory()
        .await;

    let proposal_cookie = governance_test
        .with_proposal(&token_owner_record_cookie, &mut governance_cookie)
        .await
        .unwrap();

    // Act
    let err = governance_test
        .with_signatory(
            &proposal_cookie,
            &governance_cookie,
            &token_owner_record_cookie,
        )
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(err, ProgramError::UninitializedAccount);
}

#[tokio::test]
pub async fn test_add_optional_signatory_to_proposal_with_required_signatories() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let (token_owner_record_cookie, mut governance_cookie, _, signatory) = governance_test
        .with_governance_with_required_signatory()
        .await;

    let proposal_cookie = governance_test
        .with_proposal(&token_owner_record_cookie, &mut governance_cookie)
        .await
        .unwrap();

    governance_test
        .with_signatory_record_for_required_signatory(
            &proposal_cookie,
            &governance_cookie,
            &signatory.pubkey(),
        )
        .await
        .unwrap();

    // Act
    governance_test
        .with_signatory(
            &proposal_cookie,
            &governance_cookie,
            &token_owner_record_cookie,
        )
        .await
        .unwrap();

    // Assert
    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;
    assert_eq!(proposal_account.signatories_count, 2);
}

#[tokio::test]
pub async fn test_add_non_matching_required_signatory_to_proposal_err() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let (token_owner_record_cookie, mut governance_cookie, _, signatory) = governance_test
        .with_governance_with_required_signatory()
        .await;

    let proposal_cookie = governance_test
        .with_proposal(&token_owner_record_cookie, &mut governance_cookie)
        .await
        .unwrap();

    let mut create_signatory_record_ix = add_signatory(
        &governance_test.program_id,
        &governance_cookie.address,
        &proposal_cookie.address,
        &AddSignatoryAuthority::None,
        &governance_test.bench.payer.pubkey(),
        &signatory.pubkey(),
    );

    create_signatory_record_ix.data = borsh::to_vec(&GovernanceInstruction::AddSignatory {
        signatory: Pubkey::new_unique(),
    })
    .unwrap();

    // Act
    let err = governance_test
        .bench
        .process_transaction(&[create_signatory_record_ix], Some(&[]))
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(err, GovernanceError::InvalidSignatoryAddress.into());
}
