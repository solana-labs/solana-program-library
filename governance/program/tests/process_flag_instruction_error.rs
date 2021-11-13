#![cfg(feature = "test-bpf")]

mod program_test;

use solana_program_test::tokio;

use program_test::*;
use spl_governance::{
    error::GovernanceError,
    state::enums::{InstructionExecutionStatus, ProposalState},
};

#[tokio::test]
async fn test_execute_flag_instruction_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let mut account_governance_cookie = governance_test
        .with_account_governance(
            &realm_cookie,
            &governed_account_cookie,
            &token_owner_record_cookie,
        )
        .await
        .unwrap();

    let mut proposal_cookie = governance_test
        .with_proposal(&token_owner_record_cookie, &mut account_governance_cookie)
        .await
        .unwrap();

    let signatory_record_cookie = governance_test
        .with_signatory(&proposal_cookie, &token_owner_record_cookie)
        .await
        .unwrap();

    let proposal_instruction_cookie = governance_test
        .with_nop_instruction(&mut proposal_cookie, &token_owner_record_cookie, 0, None)
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
        .advance_clock_by_min_timespan(proposal_instruction_cookie.account.hold_up_time as u64)
        .await;

    let clock = governance_test.bench.get_clock().await;

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

    let yes_option = proposal_account.options.first().unwrap();

    assert_eq!(0, yes_option.instructions_executed_count);
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
async fn test_execute_instruction_after_flagged_with_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_mint_cookie = governance_test.with_governed_mint().await;

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let mut mint_governance_cookie = governance_test
        .with_mint_governance(
            &realm_cookie,
            &governed_mint_cookie,
            &token_owner_record_cookie,
        )
        .await
        .unwrap();

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
            0,
            None,
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
        .advance_clock_by_min_timespan(proposal_instruction_cookie.account.hold_up_time as u64)
        .await;

    governance_test
        .flag_instruction_error(
            &proposal_cookie,
            &token_owner_record_cookie,
            &proposal_instruction_cookie,
        )
        .await
        .unwrap();

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

    let proposal_instruction_account = governance_test
        .get_proposal_instruction_account(&proposal_instruction_cookie.address)
        .await;

    assert_eq!(
        InstructionExecutionStatus::Success,
        proposal_instruction_account.execution_status
    );
}

#[tokio::test]
async fn test_execute_second_instruction_after_first_instruction_flagged_with_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_mint_cookie = governance_test.with_governed_mint().await;

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let mut mint_governance_cookie = governance_test
        .with_mint_governance(
            &realm_cookie,
            &governed_mint_cookie,
            &token_owner_record_cookie,
        )
        .await
        .unwrap();

    let mut proposal_cookie = governance_test
        .with_proposal(&token_owner_record_cookie, &mut mint_governance_cookie)
        .await
        .unwrap();

    let signatory_record_cookie = governance_test
        .with_signatory(&proposal_cookie, &token_owner_record_cookie)
        .await
        .unwrap();

    let proposal_instruction_cookie1 = governance_test
        .with_nop_instruction(&mut proposal_cookie, &token_owner_record_cookie, 0, None)
        .await
        .unwrap();

    let proposal_instruction_cookie2 = governance_test
        .with_mint_tokens_instruction(
            &governed_mint_cookie,
            &mut proposal_cookie,
            &token_owner_record_cookie,
            0,
            None,
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
        .advance_clock_by_min_timespan(proposal_instruction_cookie2.account.hold_up_time as u64)
        .await;

    governance_test
        .flag_instruction_error(
            &proposal_cookie,
            &token_owner_record_cookie,
            &proposal_instruction_cookie1,
        )
        .await
        .unwrap();

    // Act
    governance_test
        .execute_instruction(&proposal_cookie, &proposal_instruction_cookie2)
        .await
        .unwrap();

    // Assert

    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    assert_eq!(ProposalState::ExecutingWithErrors, proposal_account.state);
}

#[tokio::test]
async fn test_flag_instruction_error_with_instruction_already_executed_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_mint_cookie = governance_test.with_governed_mint().await;

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let mut mint_governance_cookie = governance_test
        .with_mint_governance(
            &realm_cookie,
            &governed_mint_cookie,
            &token_owner_record_cookie,
        )
        .await
        .unwrap();

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
            0,
            None,
        )
        .await
        .unwrap();

    // Add another instruction to prevent Proposal from transitioning to Competed state
    governance_test
        .with_nop_instruction(&mut proposal_cookie, &token_owner_record_cookie, 0, None)
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
        .advance_clock_by_min_timespan(proposal_instruction_cookie.account.hold_up_time as u64)
        .await;

    governance_test
        .execute_instruction(&proposal_cookie, &proposal_instruction_cookie)
        .await
        .unwrap();

    // Act

    let err = governance_test
        .flag_instruction_error(
            &proposal_cookie,
            &token_owner_record_cookie,
            &proposal_instruction_cookie,
        )
        .await
        .err()
        .unwrap();

    // Assert

    assert_eq!(err, GovernanceError::InstructionAlreadyExecuted.into());
}

#[tokio::test]
async fn test_flag_instruction_error_with_owner_or_delegate_must_sign_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    let mut token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let mut account_governance_cookie = governance_test
        .with_account_governance(
            &realm_cookie,
            &governed_account_cookie,
            &token_owner_record_cookie,
        )
        .await
        .unwrap();

    let mut proposal_cookie = governance_test
        .with_proposal(&token_owner_record_cookie, &mut account_governance_cookie)
        .await
        .unwrap();

    let signatory_record_cookie = governance_test
        .with_signatory(&proposal_cookie, &token_owner_record_cookie)
        .await
        .unwrap();

    let proposal_instruction_cookie = governance_test
        .with_nop_instruction(&mut proposal_cookie, &token_owner_record_cookie, 0, None)
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
        .advance_clock_by_min_timespan(proposal_instruction_cookie.account.hold_up_time as u64)
        .await;

    let token_owner_record_cookie2 = governance_test
        .with_council_token_deposit(&realm_cookie)
        .await
        .unwrap();

    // Try to maliciously sign using different owner signature
    token_owner_record_cookie.token_owner = token_owner_record_cookie2.token_owner;

    // Act

    let err = governance_test
        .flag_instruction_error(
            &proposal_cookie,
            &token_owner_record_cookie,
            &proposal_instruction_cookie,
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
