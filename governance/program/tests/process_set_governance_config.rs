mod program_test;

use program_test::*;
use solana_program_test::tokio;
use spl_governance::instruction::Vote;

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
