#![cfg(feature = "test-bpf")]

mod program_test;

use borsh::BorshSerialize;
use program_test::{tools::ProgramInstructionError, *};
use solana_program_test::tokio;
use solana_sdk::{signature::Keypair, signer::Signer};
use spl_governance::{
    error::GovernanceError,
    instruction::{set_governance_config, GovernanceInstruction, Vote},
};

#[tokio::test]
async fn test_set_governance_config() {
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

    let mut proposal_cookie = governance_test
        .with_proposal(&token_owner_record_cookie, &mut account_governance_cookie)
        .await
        .unwrap();

    let signatory_record_cookie = governance_test
        .with_signatory(&proposal_cookie, &token_owner_record_cookie)
        .await
        .unwrap();

    let mut governance_config =
        governance_test.get_default_governance_config(&realm_cookie, &governed_account_cookie);

    // Change vote_threshold_percentage on the Governance config
    governance_config.vote_threshold_percentage = 40;

    let proposal_instruction_cookie = governance_test
        .with_set_governance_config_instruction(
            &mut proposal_cookie,
            &token_owner_record_cookie,
            &governance_config,
        )
        .await
        .unwrap();

    governance_test
        .sign_off_proposal(&proposal_cookie, &signatory_record_cookie)
        .await
        .unwrap();

    governance_test
        .with_cast_vote(&proposal_cookie, &token_owner_record_cookie, Vote::Yes)
        .await
        .unwrap();

    // Advance timestamp past hold_up_time
    governance_test
        .advance_clock_by_min_timespan(proposal_instruction_cookie.account.hold_up_time as u64)
        .await;

    // Act
    governance_test
        .execute_instruction(&proposal_cookie, &proposal_instruction_cookie)
        .await
        .unwrap();

    // Assert
    let governance_account = governance_test
        .get_governance_account(&account_governance_cookie.address)
        .await;

    assert_eq!(governance_config, governance_account.config);
}

#[tokio::test]
async fn test_set_governance_config_with_governance_must_sign_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    let governance_config =
        governance_test.get_default_governance_config(&realm_cookie, &governed_account_cookie);

    let mut set_governance_config_ix =
        set_governance_config(&governance_test.program_id, governance_config.clone());

    // Remove governance signer from instruction
    set_governance_config_ix.accounts[1].is_signer = false;

    // Act
    let err = governance_test
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
    let governed_account_cookie = governance_test.with_governed_account().await;

    let governance_config =
        governance_test.get_default_governance_config(&realm_cookie, &governed_account_cookie);

    let mut set_governance_config_ix =
        set_governance_config(&governance_test.program_id, governance_config.clone());

    // Set Governance signer to fake account we have authority over and can use to sign the transaction
    let governance_signer = Keypair::new();
    set_governance_config_ix.accounts[1].pubkey = governance_signer.pubkey();

    // Act
    let err = governance_test
        .process_transaction(&[set_governance_config_ix], Some(&[&governance_signer]))
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(err, GovernanceError::AccountDoesNotExist.into());
}

#[tokio::test]
async fn test_set_governance_config_with_invalid_governance_authority_error() {
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

    let mut proposal_cookie = governance_test
        .with_proposal(&token_owner_record_cookie, &mut account_governance_cookie)
        .await
        .unwrap();

    let signatory_record_cookie = governance_test
        .with_signatory(&proposal_cookie, &token_owner_record_cookie)
        .await
        .unwrap();

    // Try to maliciously use a different governed account to change the given governance config
    let governed_account_cookie2 = governance_test.with_governed_account().await;

    let account_governance_cookie2 = governance_test
        .with_account_governance(&realm_cookie, &governed_account_cookie2)
        .await
        .unwrap();

    let mut governance_config =
        governance_test.get_default_governance_config(&realm_cookie, &governed_account_cookie);
    governance_config.governed_account = account_governance_cookie2.address;

    let proposal_instruction_cookie = governance_test
        .with_set_governance_config_instruction(
            &mut proposal_cookie,
            &token_owner_record_cookie,
            &governance_config,
        )
        .await
        .unwrap();

    governance_test
        .sign_off_proposal(&proposal_cookie, &signatory_record_cookie)
        .await
        .unwrap();

    governance_test
        .with_cast_vote(&proposal_cookie, &token_owner_record_cookie, Vote::Yes)
        .await
        .unwrap();

    // Advance timestamp past hold_up_time
    governance_test
        .advance_clock_by_min_timespan(proposal_instruction_cookie.account.hold_up_time as u64)
        .await;

    // Act
    let err = governance_test
        .execute_instruction(&proposal_cookie, &proposal_instruction_cookie)
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(err, ProgramInstructionError::PrivilegeEscalation.into());
}

#[tokio::test]
async fn test_set_governance_config_with_invalid_config_realm_error() {
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

    let mut proposal_cookie = governance_test
        .with_proposal(&token_owner_record_cookie, &mut account_governance_cookie)
        .await
        .unwrap();

    let signatory_record_cookie = governance_test
        .with_signatory(&proposal_cookie, &token_owner_record_cookie)
        .await
        .unwrap();

    let mut governance_config =
        governance_test.get_default_governance_config(&realm_cookie, &governed_account_cookie);

    let mut set_governance_config_ix =
        set_governance_config(&governance_test.program_id, governance_config.clone());

    // Try to maliciously change realm  in the governance config
    let realm_cookie2 = governance_test.with_realm().await;
    governance_config.realm = realm_cookie2.address;
    set_governance_config_ix.data = (GovernanceInstruction::SetGovernanceConfig {
        config: governance_config,
    })
    .try_to_vec()
    .unwrap();

    let proposal_instruction_cookie = governance_test
        .with_instruction(
            &mut proposal_cookie,
            &token_owner_record_cookie,
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
        .with_cast_vote(&proposal_cookie, &token_owner_record_cookie, Vote::Yes)
        .await
        .unwrap();

    // Advance timestamp past hold_up_time
    governance_test
        .advance_clock_by_min_timespan(proposal_instruction_cookie.account.hold_up_time as u64)
        .await;

    // Act
    let err = governance_test
        .execute_instruction(&proposal_cookie, &proposal_instruction_cookie)
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(err, GovernanceError::InvalidConfigRealmForGovernance.into());
}

#[tokio::test]
async fn test_set_governance_config_with_invalid_config_governed_account_error() {
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

    let mut proposal_cookie = governance_test
        .with_proposal(&token_owner_record_cookie, &mut account_governance_cookie)
        .await
        .unwrap();

    let signatory_record_cookie = governance_test
        .with_signatory(&proposal_cookie, &token_owner_record_cookie)
        .await
        .unwrap();

    let mut governance_config =
        governance_test.get_default_governance_config(&realm_cookie, &governed_account_cookie);

    let mut set_governance_config_ix =
        set_governance_config(&governance_test.program_id, governance_config.clone());

    // Try to maliciously change governed account  in the governance config
    let governed_account_cookie2 = governance_test.with_governed_account().await;
    governance_config.governed_account = governed_account_cookie2.address;
    set_governance_config_ix.data = (GovernanceInstruction::SetGovernanceConfig {
        config: governance_config,
    })
    .try_to_vec()
    .unwrap();

    let proposal_instruction_cookie = governance_test
        .with_instruction(
            &mut proposal_cookie,
            &token_owner_record_cookie,
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
        .with_cast_vote(&proposal_cookie, &token_owner_record_cookie, Vote::Yes)
        .await
        .unwrap();

    // Advance timestamp past hold_up_time
    governance_test
        .advance_clock_by_min_timespan(proposal_instruction_cookie.account.hold_up_time as u64)
        .await;

    // Act
    let err = governance_test
        .execute_instruction(&proposal_cookie, &proposal_instruction_cookie)
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(
        err,
        GovernanceError::InvalidConfigGovernedAccountForGovernance.into()
    );
}
