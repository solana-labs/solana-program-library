#![cfg(feature = "test-sbf")]

mod program_test;

use {
    program_test::*,
    solana_program::pubkey::Pubkey,
    solana_program_test::tokio,
    solana_sdk::signature::{Keypair, Signer},
    spl_governance::{error::GovernanceError, state::enums::ProposalState},
    spl_governance_tools::error::GovernanceToolsError,
};

#[tokio::test]
async fn test_sign_off_proposal() {
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

    let signatory_record_cookie = governance_test
        .with_signatory(
            &proposal_cookie,
            &governance_cookie,
            &token_owner_record_cookie,
        )
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

    assert!(signatory_record_account.signed_off);
}

#[tokio::test]
async fn test_sign_off_proposal_with_signatory_must_sign_error() {
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

    let signatory_record_cookie = governance_test
        .with_signatory(
            &proposal_cookie,
            &governance_cookie,
            &token_owner_record_cookie,
        )
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

    governance_test
        .with_signatory(
            &proposal_cookie,
            &governance_cookie,
            &token_owner_record_cookie,
        )
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

#[tokio::test]
async fn test_sign_off_proposal_with_non_existing_governance_error() {
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

    // Override Governance with non existing account
    proposal_cookie.account.governance = Pubkey::new_unique();

    // Act
    let err = governance_test
        .sign_off_proposal(&proposal_cookie, &signatory_record_cookie)
        .await
        .err()
        .unwrap();

    // Assert

    assert_eq!(err, GovernanceToolsError::AccountDoesNotExist.into());
}

#[tokio::test]
async fn test_sign_off_proposal_with_non_existing_realm_error() {
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

    // Override Realm with non existing account
    proposal_cookie.realm = Pubkey::new_unique();

    // Act
    let err = governance_test
        .sign_off_proposal(&proposal_cookie, &signatory_record_cookie)
        .await
        .err()
        .unwrap();

    // Assert

    assert_eq!(err, GovernanceToolsError::AccountDoesNotExist.into());
}

#[tokio::test]
async fn test_sign_off_proposal_with_required_signatory() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;

    let signatory = Keypair::new();

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
            &signatory.pubkey(),
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
            &signatory.pubkey(),
        )
        .await
        .unwrap();

    // Act
    governance_test
        .do_required_signoff(
            &realm_cookie,
            &governance_cookie,
            &new_proposal_cookie,
            &signatory,
        )
        .await
        .unwrap();

    // Assert
    let proposal_account = governance_test
        .get_proposal_account(&new_proposal_cookie.address)
        .await;

    assert_eq!(1, proposal_account.signatories_signed_off_count);
    assert_eq!(ProposalState::Voting, proposal_account.state);
}

#[tokio::test]
async fn test_partial_sign_off_proposal_with_two_governance_signatories() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;

    let signatory_1 = Keypair::new();
    let signatory_2 = Keypair::new();

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

    let proposal_transaction_cookie_1 = governance_test
        .with_add_required_signatory_transaction(
            &mut proposal_cookie,
            &token_owner_record_cookie,
            &governance_cookie,
            &signatory_1.pubkey(),
        )
        .await
        .unwrap();

    let proposal_transaction_cookie_2 = governance_test
        .with_add_required_signatory_transaction(
            &mut proposal_cookie,
            &token_owner_record_cookie,
            &governance_cookie,
            &signatory_2.pubkey(),
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

    governance_test
        .advance_clock_by_min_timespan(
            governance_cookie.account.config.transactions_hold_up_time as u64,
        )
        .await;

    governance_test
        .execute_proposal_transaction(&proposal_cookie, &proposal_transaction_cookie_1)
        .await
        .unwrap();

    governance_test
        .execute_proposal_transaction(&proposal_cookie, &proposal_transaction_cookie_2)
        .await
        .unwrap();

    // End setup proposal

    let new_proposal_cookie = governance_test
        .with_proposal(&token_owner_record_cookie, &mut governance_cookie)
        .await
        .unwrap();

    governance_test
        .with_signatory_record_for_required_signatory(
            &new_proposal_cookie,
            &governance_cookie,
            &signatory_1.pubkey(),
        )
        .await
        .unwrap();

    governance_test
        .with_signatory_record_for_required_signatory(
            &new_proposal_cookie,
            &governance_cookie,
            &signatory_2.pubkey(),
        )
        .await
        .unwrap();

    // Act
    governance_test
        .do_required_signoff(
            &realm_cookie,
            &governance_cookie,
            &new_proposal_cookie,
            &signatory_1,
        )
        .await
        .unwrap();

    // Assert
    let proposal_account = governance_test
        .get_proposal_account(&new_proposal_cookie.address)
        .await;

    assert_eq!(1, proposal_account.signatories_signed_off_count);
    assert_eq!(ProposalState::SigningOff, proposal_account.state);
}

#[tokio::test]
async fn test_repeat_sign_off_proposal_err() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;

    let signatory_1 = Keypair::new();
    let signatory_2 = Keypair::new();

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let mut governance_cookie = governance_test
        .with_governance(&realm_cookie, &token_owner_record_cookie)
        .await
        .unwrap();

    // Proposal to create required signatory 1
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
            &signatory_1.pubkey(),
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

    governance_test
        .advance_clock_by_min_timespan(
            governance_cookie.account.config.transactions_hold_up_time as u64,
        )
        .await;

    governance_test
        .execute_proposal_transaction(&proposal_cookie, &proposal_transaction_cookie)
        .await
        .unwrap();

    // Proposal to create required signatory 2
    let mut proposal_cookie = governance_test
        .with_proposal(&token_owner_record_cookie, &mut governance_cookie)
        .await
        .unwrap();

    let proposal_transaction_cookie = governance_test
        .with_add_required_signatory_transaction(
            &mut proposal_cookie,
            &token_owner_record_cookie,
            &governance_cookie,
            &signatory_2.pubkey(),
        )
        .await
        .unwrap();

    governance_test
        .with_signatory_record_for_required_signatory(
            &proposal_cookie,
            &governance_cookie,
            &signatory_1.pubkey(),
        )
        .await
        .unwrap();

    governance_test
        .do_required_signoff(
            &realm_cookie,
            &governance_cookie,
            &proposal_cookie,
            &signatory_1,
        )
        .await
        .unwrap();

    governance_test
        .with_cast_yes_no_vote(&proposal_cookie, &token_owner_record_cookie, YesNoVote::Yes)
        .await
        .unwrap();

    governance_test
        .advance_clock_by_min_timespan(
            governance_cookie.account.config.transactions_hold_up_time as u64,
        )
        .await;

    governance_test
        .execute_proposal_transaction(&proposal_cookie, &proposal_transaction_cookie)
        .await
        .unwrap();

    // End setup proposals

    let new_proposal_cookie = governance_test
        .with_proposal(&token_owner_record_cookie, &mut governance_cookie)
        .await
        .unwrap();

    governance_test
        .with_signatory_record_for_required_signatory(
            &new_proposal_cookie,
            &governance_cookie,
            &signatory_1.pubkey(),
        )
        .await
        .unwrap();

    governance_test
        .with_signatory_record_for_required_signatory(
            &new_proposal_cookie,
            &governance_cookie,
            &signatory_2.pubkey(),
        )
        .await
        .unwrap();

    // Sign off 1
    governance_test
        .do_required_signoff(
            &realm_cookie,
            &governance_cookie,
            &new_proposal_cookie,
            &signatory_1,
        )
        .await
        .unwrap();
    governance_test.advance_clock().await;

    // Act
    let err = governance_test
        .do_required_signoff(
            &realm_cookie,
            &governance_cookie,
            &new_proposal_cookie,
            &signatory_1,
        )
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(err, GovernanceError::SignatoryAlreadySignedOff.into());
}

#[tokio::test]
async fn test_sign_off_without_all_required_signatories_err() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;

    let signatory_1 = Keypair::new();
    let signatory_2 = Keypair::new();

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let mut governance_cookie = governance_test
        .with_governance(&realm_cookie, &token_owner_record_cookie)
        .await
        .unwrap();

    // Proposal to create required signatory 1
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
            &signatory_1.pubkey(),
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

    governance_test
        .advance_clock_by_min_timespan(
            governance_cookie.account.config.transactions_hold_up_time as u64,
        )
        .await;

    governance_test
        .execute_proposal_transaction(&proposal_cookie, &proposal_transaction_cookie)
        .await
        .unwrap();

    // Proposal to create required signatory 2
    let mut proposal_cookie = governance_test
        .with_proposal(&token_owner_record_cookie, &mut governance_cookie)
        .await
        .unwrap();

    let proposal_transaction_cookie = governance_test
        .with_add_required_signatory_transaction(
            &mut proposal_cookie,
            &token_owner_record_cookie,
            &governance_cookie,
            &signatory_2.pubkey(),
        )
        .await
        .unwrap();

    governance_test
        .with_signatory_record_for_required_signatory(
            &proposal_cookie,
            &governance_cookie,
            &signatory_1.pubkey(),
        )
        .await
        .unwrap();

    governance_test
        .do_required_signoff(
            &realm_cookie,
            &governance_cookie,
            &proposal_cookie,
            &signatory_1,
        )
        .await
        .unwrap();

    governance_test
        .with_cast_yes_no_vote(&proposal_cookie, &token_owner_record_cookie, YesNoVote::Yes)
        .await
        .unwrap();

    governance_test
        .advance_clock_by_min_timespan(
            governance_cookie.account.config.transactions_hold_up_time as u64,
        )
        .await;

    governance_test
        .execute_proposal_transaction(&proposal_cookie, &proposal_transaction_cookie)
        .await
        .unwrap();

    // End setup proposals

    let new_proposal_cookie = governance_test
        .with_proposal(&token_owner_record_cookie, &mut governance_cookie)
        .await
        .unwrap();

    governance_test
        .with_signatory_record_for_required_signatory(
            &new_proposal_cookie,
            &governance_cookie,
            &signatory_1.pubkey(),
        )
        .await
        .unwrap();

    // Act
    let err = governance_test
        .do_required_signoff(
            &realm_cookie,
            &governance_cookie,
            &new_proposal_cookie,
            &signatory_1,
        )
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(err, GovernanceError::MissingRequiredSignatories.into());
}
