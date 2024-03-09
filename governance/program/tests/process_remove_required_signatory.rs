#![cfg(feature = "test-sbf")]

mod program_test;

use {
    program_test::*,
    solana_program::pubkey::Pubkey,
    solana_program_test::tokio,
    solana_sdk::signature::Signer,
    spl_governance::{error::GovernanceError, instruction::remove_required_signatory},
    spl_governance_tools::error::GovernanceToolsError,
};

#[tokio::test]
async fn test_remove_required_signatory() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let (token_owner_record_cookie, mut governance_cookie, realm_cookie, signatory) =
        governance_test
            .with_governance_with_required_signatory()
            .await;

    let mut proposal_cookie = governance_test
        .with_proposal(&token_owner_record_cookie, &mut governance_cookie)
        .await
        .unwrap();

    let beneficiary = Pubkey::new_unique();

    let proposal_transaction_cookie = governance_test
        .with_remove_required_signatory_transaction(
            &mut proposal_cookie,
            &token_owner_record_cookie,
            &governance_cookie,
            &signatory.pubkey(),
            &beneficiary,
        )
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

    governance_test
        .do_required_signoff(
            &realm_cookie,
            &governance_cookie,
            &proposal_cookie,
            &signatory,
        )
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

    // Act
    governance_test
        .execute_proposal_transaction(&proposal_cookie, &proposal_transaction_cookie)
        .await
        .unwrap();

    // Assert
    let after_proposal_cookie = governance_test
        .with_proposal(&token_owner_record_cookie, &mut governance_cookie)
        .await
        .unwrap();
    let proposal_account = governance_test
        .get_proposal_account(&after_proposal_cookie.address)
        .await;

    assert_eq!(0, proposal_account.signatories_count);

    let governance_account = governance_test
        .get_governance_account(&governance_cookie.address)
        .await;

    assert_eq!(0, governance_account.required_signatories_count);
}

#[tokio::test]
async fn test_remove_non_existing_required_signatory_err() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let (token_owner_record_cookie, mut governance_cookie, realm_cookie, signatory) =
        governance_test
            .with_governance_with_required_signatory()
            .await;

    let mut proposal_cookie = governance_test
        .with_proposal(&token_owner_record_cookie, &mut governance_cookie)
        .await
        .unwrap();

    let beneficiary = Pubkey::new_unique();

    let proposal_transaction_cookie = governance_test
        .with_remove_required_signatory_transaction(
            &mut proposal_cookie,
            &token_owner_record_cookie,
            &governance_cookie,
            &Pubkey::new_unique(),
            &beneficiary,
        )
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

    governance_test
        .do_required_signoff(
            &realm_cookie,
            &governance_cookie,
            &proposal_cookie,
            &signatory,
        )
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

    // Act
    let err = governance_test
        .execute_proposal_transaction(&proposal_cookie, &proposal_transaction_cookie)
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(err, GovernanceToolsError::AccountDoesNotExist.into());
}

#[tokio::test]
pub async fn remove_required_signatory_from_governance_without_governance_signer_err() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;

    let signatory = Pubkey::new_unique();

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let governance_cookie = governance_test
        .with_governance(&realm_cookie, &token_owner_record_cookie)
        .await
        .unwrap();

    let mut gwr_ix = remove_required_signatory(
        &governance_test.program_id,
        &governance_cookie.address,
        &signatory,
        &governance_test.bench.payer.pubkey(),
    );

    gwr_ix.accounts[0].is_signer = false;

    // Act
    let err = governance_test
        .bench
        .process_transaction(&[gwr_ix], Some(&[]))
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(err, GovernanceError::GovernancePdaMustSign.into());
}
