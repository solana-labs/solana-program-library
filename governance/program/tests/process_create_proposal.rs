#![cfg(feature = "test-bpf")]

use solana_program::instruction::AccountMeta;
use solana_program_test::*;

mod program_test;

use program_test::*;
use solana_sdk::signature::Keypair;
use spl_governance::error::GovernanceError;

#[tokio::test]
async fn test_community_proposal_created() {
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

    // Act
    let proposal_cookie = governance_test
        .with_proposal(&token_owner_record_cookie, &mut account_governance_cookie)
        .await
        .unwrap();

    // Assert
    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    assert_eq!(proposal_cookie.account, proposal_account);

    let account_governance_account = governance_test
        .get_governance_account(&account_governance_cookie.address)
        .await;

    assert_eq!(1, account_governance_account.proposals_count);
    assert_eq!(proposal_account.draft_at, 1);
}

#[tokio::test]
async fn test_multiple_proposals_created() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    let mut account_governance_cookie = governance_test
        .with_account_governance(&realm_cookie, &governed_account_cookie)
        .await
        .unwrap();

    let community_token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await;

    let council_token_owner_record_cookie = governance_test
        .with_council_token_deposit(&realm_cookie)
        .await;

    // Act
    let community_proposal_cookie = governance_test
        .with_proposal(
            &community_token_owner_record_cookie,
            &mut account_governance_cookie,
        )
        .await
        .unwrap();

    let council_proposal_cookie = governance_test
        .with_proposal(
            &council_token_owner_record_cookie,
            &mut account_governance_cookie,
        )
        .await
        .unwrap();

    // Assert
    let community_proposal_account = governance_test
        .get_proposal_account(&community_proposal_cookie.address)
        .await;

    assert_eq!(
        community_proposal_cookie.account,
        community_proposal_account
    );

    let council_proposal_account = governance_test
        .get_proposal_account(&council_proposal_cookie.address)
        .await;

    assert_eq!(council_proposal_cookie.account, council_proposal_account);

    let account_governance_account = governance_test
        .get_governance_account(&account_governance_cookie.address)
        .await;

    assert_eq!(2, account_governance_account.proposals_count);
}

#[tokio::test]
async fn test_create_proposal_with_not_authorized_governance_authority_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    let mut account_governance_cookie = governance_test
        .with_account_governance(&realm_cookie, &governed_account_cookie)
        .await
        .unwrap();

    let mut token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await;

    token_owner_record_cookie.governance_authority = Some(Keypair::new());

    // Act
    let err = governance_test
        .with_proposal(&token_owner_record_cookie, &mut account_governance_cookie)
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
async fn test_create_proposal_with_governance_delegate_signer() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    let mut account_governance_cookie = governance_test
        .with_account_governance(&realm_cookie, &governed_account_cookie)
        .await
        .unwrap();

    let mut token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await;
    governance_test
        .with_community_governance_delegate(&realm_cookie, &mut token_owner_record_cookie)
        .await;

    token_owner_record_cookie.governance_authority =
        Some(token_owner_record_cookie.clone_governance_delegate());

    // Act
    let proposal_cookie = governance_test
        .with_proposal(&token_owner_record_cookie, &mut account_governance_cookie)
        .await
        .unwrap();

    // Assert
    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    assert_eq!(proposal_cookie.account, proposal_account);
}

#[tokio::test]
async fn test_create_proposal_with_not_enough_tokens_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    let mut account_governance_cookie = governance_test
        .with_account_governance(&realm_cookie, &governed_account_cookie)
        .await
        .unwrap();

    let token_amount = account_governance_cookie
        .account
        .config
        .min_tokens_to_create_proposal as u64
        - 1;

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit_amount(&realm_cookie, token_amount)
        .await;

    // Act
    let err = governance_test
        .with_proposal(&token_owner_record_cookie, &mut account_governance_cookie)
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(err, GovernanceError::NotEnoughTokensToCreateProposal.into());
}

#[tokio::test]
async fn test_create_proposal_with_invalid_token_owner_record_error() {
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

    let council_token_owner_record_cookie = governance_test
        .with_council_token_deposit(&realm_cookie)
        .await;

    // Act
    let err = governance_test
        .with_proposal_using_instruction(
            &token_owner_record_cookie,
            &mut account_governance_cookie,
            |i| {
                // Set token_owner_record_address for different (Council) mint
                i.accounts[2] =
                    AccountMeta::new_readonly(council_token_owner_record_cookie.address, false);
            },
        )
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(
        err,
        GovernanceError::InvalidGoverningMintForTokenOwnerRecord.into()
    );
}
