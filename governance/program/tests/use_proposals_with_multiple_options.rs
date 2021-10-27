#![cfg(feature = "test-bpf")]

use solana_program_test::*;

mod program_test;

use program_test::*;
use spl_governance::{error::GovernanceError, state::proposal::VoteType};

#[tokio::test]
async fn test_create_proposal_with_single_choice_options_and_reject_option() {
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

    let options = vec!["option 1".to_string(), "option 2".to_string()];

    // Act
    let proposal_cookie = governance_test
        .with_multi_option_proposal(
            &token_owner_record_cookie,
            &mut account_governance_cookie,
            options,
            true,
            VoteType::SingleChoice,
        )
        .await
        .unwrap();

    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    assert_eq!(proposal_account.vote_type, VoteType::SingleChoice);
    assert!(proposal_account.reject_option_vote_weight.is_some());

    assert_eq!(proposal_cookie.account, proposal_account);
}

#[tokio::test]
async fn test_create_proposal_with_multiple_choice_options_and_without_reject_option() {
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

    let options = vec!["option 1".to_string(), "option 2".to_string()];

    // Act
    let proposal_cookie = governance_test
        .with_multi_option_proposal(
            &token_owner_record_cookie,
            &mut account_governance_cookie,
            options,
            false,
            VoteType::MultiChoice,
        )
        .await
        .unwrap();

    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    assert_eq!(proposal_account.vote_type, VoteType::MultiChoice);
    assert!(!proposal_account.reject_option_vote_weight.is_some());

    assert_eq!(proposal_cookie.account, proposal_account);
}

#[tokio::test]
async fn test_insert_instruction_with_proposal_not_executable_error() {
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
        .with_multi_option_proposal(
            &token_owner_record_cookie,
            &mut account_governance_cookie,
            vec!["option 1".to_string(), "option 2".to_string()],
            false,
            VoteType::SingleChoice,
        )
        .await
        .unwrap();

    // Act
    let err = governance_test
        .with_nop_instruction(&mut proposal_cookie, &token_owner_record_cookie, 0, None)
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(err, GovernanceError::ProposalIsNotExecutable.into());
}

#[tokio::test]
async fn test_insert_instructions_for_multiple_options() {
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
        .with_multi_option_proposal(
            &token_owner_record_cookie,
            &mut account_governance_cookie,
            vec!["option 1".to_string(), "option 2".to_string()],
            true,
            VoteType::SingleChoice,
        )
        .await
        .unwrap();

    // Act

    // option 1 / instruction 0
    governance_test
        .with_nop_instruction(&mut proposal_cookie, &token_owner_record_cookie, 1, Some(0))
        .await
        .unwrap();

    // option 1 / instruction 1
    governance_test
        .with_nop_instruction(&mut proposal_cookie, &token_owner_record_cookie, 1, Some(1))
        .await
        .unwrap();

    // option 1 / instruction 2
    governance_test
        .with_nop_instruction(&mut proposal_cookie, &token_owner_record_cookie, 1, Some(2))
        .await
        .unwrap();

    // option 0 / instruction 0
    governance_test
        .with_nop_instruction(&mut proposal_cookie, &token_owner_record_cookie, 0, Some(0))
        .await
        .unwrap();

    // option 0 / instruction 1
    governance_test
        .with_nop_instruction(&mut proposal_cookie, &token_owner_record_cookie, 0, Some(1))
        .await
        .unwrap();

    // Assert
    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    assert_eq!(2, proposal_account.options[0].instructions_count);
    assert_eq!(3, proposal_account.options[1].instructions_count);
}
