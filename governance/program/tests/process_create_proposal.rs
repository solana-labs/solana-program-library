#![cfg(feature = "test-bpf")]

use solana_program::{instruction::AccountMeta, pubkey::Pubkey};
use solana_program_test::*;

mod program_test;

use program_test::*;
use solana_sdk::signature::Keypair;
use spl_governance::error::GovernanceError;

#[tokio::test]
async fn test_create_community_proposal() {
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

    let token_owner_record_account = governance_test
        .get_token_owner_record_account(&token_owner_record_cookie.address)
        .await;

    assert_eq!(1, token_owner_record_account.outstanding_proposal_count);
}

#[tokio::test]
async fn test_create_multiple_proposals() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    let community_token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let mut account_governance_cookie = governance_test
        .with_account_governance(
            &realm_cookie,
            &governed_account_cookie,
            &community_token_owner_record_cookie,
        )
        .await
        .unwrap();

    let council_token_owner_record_cookie = governance_test
        .with_council_token_deposit(&realm_cookie)
        .await
        .unwrap();

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
async fn test_create_proposal_with_not_enough_community_tokens_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    let token_owner_record_cookie1 = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let mut account_governance_cookie = governance_test
        .with_account_governance(
            &realm_cookie,
            &governed_account_cookie,
            &token_owner_record_cookie1,
        )
        .await
        .unwrap();

    // Set token deposit amount below the required threshold
    let token_amount = 4;

    let token_owner_record_cookie2 = governance_test
        .with_community_token_deposit_amount(&realm_cookie, token_amount)
        .await
        .unwrap();

    // Act
    let err = governance_test
        .with_proposal(&token_owner_record_cookie2, &mut account_governance_cookie)
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(err, GovernanceError::NotEnoughTokensToCreateProposal.into());
}

#[tokio::test]
async fn test_create_proposal_with_not_enough_council_tokens_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    // Set token deposit amount below the required threshold
    let token_amount = 1;

    let token_owner_record_cookie = governance_test
        .with_council_token_deposit_amount(&realm_cookie, token_amount)
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
async fn test_create_proposal_with_owner_or_delegate_must_sign_error() {
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

    let council_token_owner_record_cookie = governance_test
        .with_council_token_deposit(&realm_cookie)
        .await
        .unwrap();

    // Act
    let err = governance_test
        .with_proposal_using_instruction(
            &token_owner_record_cookie,
            &mut account_governance_cookie,
            |i| {
                // Set token_owner_record_address for different (Council) mint
                i.accounts[3] =
                    AccountMeta::new_readonly(council_token_owner_record_cookie.address, false);
            },
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
async fn test_create_proposal_with_invalid_governing_token_mint_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    let mut token_owner_record_cookie = governance_test
        .with_council_token_deposit(&realm_cookie)
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

    // Try to use mint which  doesn't belong to the Realm
    token_owner_record_cookie.account.governing_token_mint = Pubkey::new_unique();

    // Act
    let err = governance_test
        .with_proposal(&token_owner_record_cookie, &mut account_governance_cookie)
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(err, GovernanceError::InvalidGoverningTokenMint.into());
}

#[tokio::test]
async fn test_create_community_proposal_using_council_tokens() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    let mut community_token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let mut account_governance_cookie = governance_test
        .with_account_governance(
            &realm_cookie,
            &governed_account_cookie,
            &community_token_owner_record_cookie,
        )
        .await
        .unwrap();

    let council_token_owner_record_cookie = governance_test
        .with_council_token_deposit(&realm_cookie)
        .await
        .unwrap();

    // Change the proposal owner to council token owner
    community_token_owner_record_cookie.address = council_token_owner_record_cookie.address;
    community_token_owner_record_cookie.token_owner = council_token_owner_record_cookie.token_owner;

    // Act
    let proposal_cookie = governance_test
        .with_proposal(
            &community_token_owner_record_cookie,
            &mut account_governance_cookie,
        )
        .await
        .unwrap();

    // Assert
    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    assert_eq!(
        realm_cookie.account.community_mint,
        proposal_account.governing_token_mint
    );

    assert_eq!(
        council_token_owner_record_cookie.address,
        proposal_account.token_owner_record
    );
}

#[tokio::test]
async fn test_create_council_proposal_using_community_tokens() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    let mut council_token_owner_record_cookie = governance_test
        .with_council_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let mut account_governance_cookie = governance_test
        .with_account_governance(
            &realm_cookie,
            &governed_account_cookie,
            &council_token_owner_record_cookie,
        )
        .await
        .unwrap();

    let community_token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    // Change the proposal owner to community token owner
    council_token_owner_record_cookie.address = community_token_owner_record_cookie.address;
    council_token_owner_record_cookie.token_owner = community_token_owner_record_cookie.token_owner;

    // Act
    let proposal_cookie = governance_test
        .with_proposal(
            &council_token_owner_record_cookie,
            &mut account_governance_cookie,
        )
        .await
        .unwrap();

    // Assert
    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    assert_eq!(
        realm_cookie.account.config.council_mint.unwrap(),
        proposal_account.governing_token_mint
    );

    assert_eq!(
        community_token_owner_record_cookie.address,
        proposal_account.token_owner_record
    );
}
