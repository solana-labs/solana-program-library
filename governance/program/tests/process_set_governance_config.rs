#![cfg(feature = "test-bpf")]

mod program_test;

use program_test::*;
use solana_program_test::tokio;
use solana_sdk::{signature::Keypair, signer::Signer};
use spl_governance::{
    error::GovernanceError, instruction::set_governance_config,
    state::enums::VoteThresholdPercentage,
};
use spl_governance_test_sdk::tools::ProgramInstructionError;

use spl_governance_tools::error::GovernanceToolsError;

#[tokio::test]
async fn test_set_governance_config() {
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

    let mut new_governance_config = governance_test.get_default_governance_config();

    // Change vote_threshold_percentage on the new Governance config
    new_governance_config.vote_threshold_percentage = VoteThresholdPercentage::YesVote(40);

    let proposal_transaction_cookie = governance_test
        .with_set_governance_config_transaction(
            &mut proposal_cookie,
            &token_owner_record_cookie,
            &new_governance_config,
        )
        .await
        .unwrap();

    governance_test
        .sign_off_proposal(&proposal_cookie, &signatory_record_cookie)
        .await
        .unwrap();

    governance_test
        .with_cast_vote(&proposal_cookie, &token_owner_record_cookie, YesNoVote::Yes)
        .await
        .unwrap();

    // Advance timestamp past hold_up_time
    governance_test
        .advance_clock_by_min_timespan(proposal_transaction_cookie.account.hold_up_time as u64)
        .await;

    // Act
    governance_test
        .execute_proposal_transaction(&proposal_cookie, &proposal_transaction_cookie)
        .await
        .unwrap();

    // Assert
    let governance_account = governance_test
        .get_governance_account(&governance_cookie.address)
        .await;

    assert_eq!(new_governance_config, governance_account.config);
}

#[tokio::test]
async fn test_set_governance_config_with_governance_must_sign_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;

    let new_governance_config = governance_test.get_default_governance_config();

    let mut set_governance_config_ix = set_governance_config(
        &governance_test.program_id,
        &realm_cookie.address,
        new_governance_config.clone(),
    );

    // Remove governance signer from instruction
    set_governance_config_ix.accounts[0].is_signer = false;

    // Act
    let err = governance_test
        .bench
        .process_transaction(&[set_governance_config_ix], None)
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(err, GovernanceError::GovernancePdaMustSign.into());
}

#[tokio::test]
async fn test_set_governance_config_with_fake_governance_signer_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;

    let new_governance_config = governance_test.get_default_governance_config();

    let mut set_governance_config_ix = set_governance_config(
        &governance_test.program_id,
        &realm_cookie.address,
        new_governance_config.clone(),
    );

    // Set Governance signer to fake account we have authority over and can use to sign the transaction
    let governance_signer = Keypair::new();
    set_governance_config_ix.accounts[0].pubkey = governance_signer.pubkey();

    // Act
    let err = governance_test
        .bench
        .process_transaction(&[set_governance_config_ix], Some(&[&governance_signer]))
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(err, GovernanceToolsError::AccountDoesNotExist.into());
}

#[tokio::test]
async fn test_set_governance_config_with_invalid_governance_authority_error() {
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

    // Try to maliciously use a different governance account to change the given governance config
    let governed_account_cookie2 = governance_test.with_governed_account().await;

    let governance_cookie2 = governance_test
        .with_governance(
            &realm_cookie,
            &governed_account_cookie2,
            &token_owner_record_cookie,
        )
        .await
        .unwrap();

    let new_governance_config = governance_test.get_default_governance_config();

    let mut set_governance_config_ix = set_governance_config(
        &governance_test.program_id,
        &governance_cookie2.address,
        new_governance_config,
    );

    let proposal_transaction_cookie = governance_test
        .with_proposal_transaction(
            &mut proposal_cookie,
            &token_owner_record_cookie,
            0,
            None,
            &mut set_governance_config_ix,
        )
        .await
        .unwrap();

    governance_test
        .sign_off_proposal(&proposal_cookie, &signatory_record_cookie)
        .await
        .unwrap();

    governance_test
        .with_cast_vote(&proposal_cookie, &token_owner_record_cookie, YesNoVote::Yes)
        .await
        .unwrap();

    // Advance timestamp past hold_up_time
    governance_test
        .advance_clock_by_min_timespan(proposal_transaction_cookie.account.hold_up_time as u64)
        .await;

    // Act
    let err = governance_test
        .execute_proposal_transaction(&proposal_cookie, &proposal_transaction_cookie)
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(err, ProgramInstructionError::PrivilegeEscalation.into());
}
