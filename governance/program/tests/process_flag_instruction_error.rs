#![cfg(feature = "test-bpf")]

mod program_test;

use solana_program::{
    instruction::{AccountMeta, Instruction},
    program_error::ProgramError,
    sysvar::{clock, fees},
};
use solana_program_test::tokio;

use program_test::*;
use spl_governance::{
    error::GovernanceError,
    instruction::Vote,
    state::enums::{InstructionExecutionStatus, ProposalState},
};

#[tokio::test]
async fn test_execute_flag_instruction_error() {
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

    let proposal_instruction_cookie = governance_test
        .with_nop_instruction(&mut proposal_cookie, &token_owner_record_cookie, None)
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

    let clock = governance_test.get_clock().await;

    // Act
    governance_test
        .flag_instruction_error(
            &proposal_cookie,
            &token_owner_record_cookie,
            &proposal_instruction_cookie,
        )
        .await
        .unwrap();

    // Assert

    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    assert_eq!(0, proposal_account.instructions_executed_count);
    assert_eq!(ProposalState::ExecutingWithErrors, proposal_account.state);
    assert_eq!(None, proposal_account.closed_at);
    assert_eq!(Some(clock.unix_timestamp), proposal_account.executing_at);

    let proposal_instruction_account = governance_test
        .get_proposal_instruction_account(&proposal_instruction_cookie.address)
        .await;

    assert_eq!(None, proposal_instruction_account.executed_at);

    assert_eq!(
        InstructionExecutionStatus::Error,
        proposal_instruction_account.execution_status
    );
}

#[tokio::test]
async fn test_execute_instruction_with_invalid_state_errors() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_mint_cookie = governance_test.with_governed_mint().await;

    let mut mint_governance_cookie = governance_test
        .with_mint_governance(&realm_cookie, &governed_mint_cookie)
        .await
        .unwrap();

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await;

    let mut proposal_cookie = governance_test
        .with_proposal(&token_owner_record_cookie, &mut mint_governance_cookie)
        .await
        .unwrap();

    let signatory_record_cookie1 = governance_test
        .with_signatory(&proposal_cookie, &token_owner_record_cookie)
        .await
        .unwrap();

    let signatory_record_cookie2 = governance_test
        .with_signatory(&proposal_cookie, &token_owner_record_cookie)
        .await
        .unwrap();

    let proposal_instruction_cookie = governance_test
        .with_mint_tokens_instruction(
            &governed_mint_cookie,
            &mut proposal_cookie,
            &token_owner_record_cookie,
            None,
        )
        .await
        .unwrap();

    // Act

    let err = governance_test
        .execute_instruction(&proposal_cookie, &proposal_instruction_cookie)
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(
        err,
        GovernanceError::InvalidStateCannotExecuteInstruction.into()
    );

    // Arrange

    governance_test
        .sign_off_proposal(&proposal_cookie, &signatory_record_cookie1)
        .await
        .unwrap();

    // Act

    let err = governance_test
        .execute_instruction(&proposal_cookie, &proposal_instruction_cookie)
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(
        err,
        GovernanceError::InvalidStateCannotExecuteInstruction.into()
    );

    // Arrange

    governance_test
        .sign_off_proposal(&proposal_cookie, &signatory_record_cookie2)
        .await
        .unwrap();

    // Act

    let err = governance_test
        .execute_instruction(&proposal_cookie, &proposal_instruction_cookie)
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(
        err,
        GovernanceError::InvalidStateCannotExecuteInstruction.into()
    );

    // Arrange

    governance_test
        .with_cast_vote(&proposal_cookie, &token_owner_record_cookie, Vote::Yes)
        .await
        .unwrap();

    governance_test.advance_clock().await;

    // Act
    let err = governance_test
        .execute_instruction(&proposal_cookie, &proposal_instruction_cookie)
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(
        err,
        GovernanceError::CannotExecuteInstructionWithinHoldUpTime.into()
    );

    // Arrange
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

    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    assert_eq!(ProposalState::Completed, proposal_account.state);

    // Arrange

    governance_test.advance_clock().await;

    // Act
    let err = governance_test
        .execute_instruction(&proposal_cookie, &proposal_instruction_cookie)
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(
        err,
        GovernanceError::InvalidStateCannotExecuteInstruction.into()
    );
}

#[tokio::test]
async fn test_execute_instruction_for_other_proposal_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_mint_cookie = governance_test.with_governed_mint().await;

    let mut mint_governance_cookie = governance_test
        .with_mint_governance(&realm_cookie, &governed_mint_cookie)
        .await
        .unwrap();

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await;

    let mut proposal_cookie = governance_test
        .with_proposal(&token_owner_record_cookie, &mut mint_governance_cookie)
        .await
        .unwrap();

    let signatory_record_cookie = governance_test
        .with_signatory(&proposal_cookie, &token_owner_record_cookie)
        .await
        .unwrap();

    let proposal_instruction_cookie = governance_test
        .with_mint_tokens_instruction(
            &governed_mint_cookie,
            &mut proposal_cookie,
            &token_owner_record_cookie,
            None,
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

    // Advance clock past hold_up_time

    governance_test
        .advance_clock_by_min_timespan(proposal_instruction_cookie.account.hold_up_time as u64)
        .await;

    let proposal_cookie2 = governance_test
        .with_proposal(&token_owner_record_cookie, &mut mint_governance_cookie)
        .await
        .unwrap();

    // Act
    let err = governance_test
        .execute_instruction(&proposal_cookie2, &proposal_instruction_cookie)
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(
        err,
        GovernanceError::InvalidProposalForProposalInstruction.into()
    );
}

#[tokio::test]
async fn test_execute_mint_instruction_twice_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_mint_cookie = governance_test.with_governed_mint().await;

    let mut mint_governance_cookie = governance_test
        .with_mint_governance(&realm_cookie, &governed_mint_cookie)
        .await
        .unwrap();

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await;

    let mut proposal_cookie = governance_test
        .with_proposal(&token_owner_record_cookie, &mut mint_governance_cookie)
        .await
        .unwrap();

    let signatory_record_cookie = governance_test
        .with_signatory(&proposal_cookie, &token_owner_record_cookie)
        .await
        .unwrap();

    let proposal_instruction_cookie = governance_test
        .with_mint_tokens_instruction(
            &governed_mint_cookie,
            &mut proposal_cookie,
            &token_owner_record_cookie,
            None,
        )
        .await
        .unwrap();

    governance_test
        .with_nop_instruction(&mut proposal_cookie, &token_owner_record_cookie, None)
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

    // Advance clock past hold_up_time

    governance_test
        .advance_clock_by_min_timespan(proposal_instruction_cookie.account.hold_up_time as u64)
        .await;

    governance_test
        .execute_instruction(&proposal_cookie, &proposal_instruction_cookie)
        .await
        .unwrap();

    governance_test.advance_clock().await;

    // Act

    let err = governance_test
        .execute_instruction(&proposal_cookie, &proposal_instruction_cookie)
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(err, GovernanceError::InstructionAlreadyExecuted.into());
}
