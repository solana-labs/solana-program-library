#![cfg(feature = "test-sbf")]

mod program_test;

use solana_program_test::tokio;

use program_test::*;
use spl_governance::state::enums::ProposalState;
use spl_governance::{error::GovernanceError, state::proposal::VoteType};
use spl_governance_test_sdk::tools::NopOverride;

#[tokio::test]
async fn test_add_proposal_option() {
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

    // Act
    let options = vec!["Option 1".to_string(), "Option 2".to_string()];

    let mut proposal_cookie = governance_test
        .with_proposal_using_instruction_impl(
            &token_owner_record_cookie,
            &mut governance_cookie,
            vec![],
            true,
            VoteType::SingleChoice,
            NopOverride,
        )
        .await
        .unwrap();

    // on creation of the proposal there is no option inserted; no option on creation permitted
    assert_eq!(0, proposal_cookie.account.options.len());

    governance_test
        .with_proposal_options_using_instruction(
            &mut proposal_cookie,
            &token_owner_record_cookie,
            options,
            NopOverride,
        )
        .await
        .unwrap();
    assert_eq!(2, proposal_cookie.account.options.len());

    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;
    assert_eq!(2, proposal_account.options.len());
}

#[tokio::test]
async fn test_add_multi_proposal_option() {
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

    // Act
    let options_set1 = vec!["Option 1".to_string()];
    let options_set2 = vec!["Option 1".to_string()];
    let options_set3 = vec!["Option 3".to_string()];

    let mut proposal_cookie = governance_test
        .with_proposal_using_instruction_impl(
            &token_owner_record_cookie,
            &mut governance_cookie,
            options_set1,
            true,
            VoteType::SingleChoice,
            NopOverride,
        )
        .await
        .unwrap();

    assert_eq!(1, proposal_cookie.account.options.len());

    governance_test
        .with_proposal_options_using_instruction(
            &mut proposal_cookie,
            &token_owner_record_cookie,
            options_set2,
            NopOverride,
        )
        .await
        .unwrap();
    assert_eq!(2, proposal_cookie.account.options.len());

    governance_test
        .with_proposal_options_using_instruction(
            &mut proposal_cookie,
            &token_owner_record_cookie,
            options_set3,
            NopOverride,
        )
        .await
        .unwrap();
    assert_eq!(3, proposal_cookie.account.options.len());

    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;
    assert_eq!(3, proposal_account.options.len());
}

#[tokio::test]
async fn test_add_proposal_option_with_not_editable_proposal_error() {
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

    let options = vec!["Option 1".to_string()];

    let mut proposal_cookie = governance_test
        .with_proposal_using_instruction_impl(
            &token_owner_record_cookie,
            &mut governance_cookie,
            vec![],
            true,
            VoteType::SingleChoice,
            NopOverride,
        )
        .await
        .unwrap();

    governance_test
        .with_proposal_options_using_instruction(
            &mut proposal_cookie,
            &token_owner_record_cookie,
            options.clone(),
            NopOverride,
        )
        .await
        .unwrap();
    assert_eq!(1, proposal_cookie.account.options.len());

    governance_test
        .sign_off_proposal_by_owner(&proposal_cookie, &token_owner_record_cookie)
        .await
        .unwrap();
    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;
    assert_eq!(ProposalState::Voting, proposal_account.state);

    // Act
    let err = governance_test
        .with_proposal_options_using_instruction(
            &mut proposal_cookie,
            &token_owner_record_cookie,
            options.clone(),
            NopOverride,
        )
        .await
        .err()
        .unwrap();

    assert_eq!(
        err,
        GovernanceError::InvalidStateCannotEditTransactions.into()
    );
}

#[tokio::test]
async fn test_sing_off_without_inserting_proposal_option_error() {
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
        .with_proposal_using_instruction_impl(
            &token_owner_record_cookie,
            &mut governance_cookie,
            vec![],
            true,
            VoteType::SingleChoice,
            NopOverride,
        )
        .await
        .unwrap();

    let err = governance_test
        .sign_off_proposal_by_owner(&proposal_cookie, &token_owner_record_cookie)
        .await
        .err()
        .unwrap();

    assert_eq!(
        err,
        GovernanceError::AtLeastOneOptionInProposalRequired.into()
    );
}
