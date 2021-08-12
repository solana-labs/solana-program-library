#![cfg(feature = "test-bpf")]

use solana_program::{instruction::AccountMeta, pubkey::Pubkey};
use solana_program_test::*;

mod program_test;

use program_test::*;
use solana_sdk::signature::Signer;

use spl_governance::{
    error::GovernanceError,
    instruction::{withdraw_governing_tokens, Vote},
    state::token_owner_record::get_token_owner_record_address,
};

#[tokio::test]
async fn test_withdraw_community_tokens() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;
    let realm_cookie = governance_test.with_realm().await;

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await;

    // Act
    governance_test
        .withdraw_community_tokens(&realm_cookie, &token_owner_record_cookie)
        .await
        .unwrap();

    // Assert
    let token_owner_record = governance_test
        .get_token_owner_record_account(&token_owner_record_cookie.address)
        .await;

    assert_eq!(0, token_owner_record.governing_token_deposit_amount);

    let holding_account = governance_test
        .get_token_account(&realm_cookie.community_token_holding_account)
        .await;

    assert_eq!(0, holding_account.amount);

    let source_account = governance_test
        .get_token_account(&token_owner_record_cookie.token_source)
        .await;

    assert_eq!(
        token_owner_record_cookie.token_source_amount,
        source_account.amount
    );
}

#[tokio::test]
async fn test_withdraw_council_tokens() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;
    let realm_cookie = governance_test.with_realm().await;

    let token_owner_record_cookie = governance_test
        .with_council_token_deposit(&realm_cookie)
        .await;

    // Act
    governance_test
        .withdraw_council_tokens(&realm_cookie, &token_owner_record_cookie)
        .await
        .unwrap();

    // Assert
    let token_owner_record = governance_test
        .get_token_owner_record_account(&token_owner_record_cookie.address)
        .await;

    assert_eq!(0, token_owner_record.governing_token_deposit_amount);

    let holding_account = governance_test
        .get_token_account(&realm_cookie.council_token_holding_account.unwrap())
        .await;

    assert_eq!(0, holding_account.amount);

    let source_account = governance_test
        .get_token_account(&token_owner_record_cookie.token_source)
        .await;

    assert_eq!(
        token_owner_record_cookie.token_source_amount,
        source_account.amount
    );
}

#[tokio::test]
async fn test_withdraw_community_tokens_with_owner_must_sign_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;
    let realm_cookie = governance_test.with_realm().await;

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await;

    let hacker_token_destination = Pubkey::new_unique();

    let mut instruction = withdraw_governing_tokens(
        &governance_test.program_id,
        &realm_cookie.address,
        &hacker_token_destination,
        &token_owner_record_cookie.token_owner.pubkey(),
        &realm_cookie.account.community_mint,
    );

    instruction.accounts[3] =
        AccountMeta::new_readonly(token_owner_record_cookie.token_owner.pubkey(), false);

    // Act
    let err = governance_test
        .process_transaction(&[instruction], None)
        .await
        .err()
        .unwrap();

    // Assert

    assert_eq!(err, GovernanceError::GoverningTokenOwnerMustSign.into());
}

#[tokio::test]
async fn test_withdraw_community_tokens_with_token_owner_record_address_mismatch_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;
    let realm_cookie = governance_test.with_realm().await;

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await;

    let vote_record_address = get_token_owner_record_address(
        &governance_test.program_id,
        &realm_cookie.address,
        &realm_cookie.account.community_mint,
        &token_owner_record_cookie.token_owner.pubkey(),
    );

    let hacker_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await;

    let mut instruction = withdraw_governing_tokens(
        &governance_test.program_id,
        &realm_cookie.address,
        &hacker_record_cookie.token_source,
        &hacker_record_cookie.token_owner.pubkey(),
        &realm_cookie.account.community_mint,
    );

    instruction.accounts[4] = AccountMeta::new(vote_record_address, false);

    // Act
    let err = governance_test
        .process_transaction(&[instruction], Some(&[&hacker_record_cookie.token_owner]))
        .await
        .err()
        .unwrap();

    // Assert

    assert_eq!(
        err,
        GovernanceError::InvalidTokenOwnerRecordAccountAddress.into()
    );
}

#[tokio::test]
async fn test_withdraw_governing_tokens_with_unrelinquished_votes_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await;
    let mut account_governance_cookie = governance_test
        .with_account_governance(
            &realm_cookie,
            &governed_account_cookie,
            &token_owner_record_cookie,
        )
        .await
        .unwrap();

    let proposal_cookie = governance_test
        .with_signed_off_proposal(&token_owner_record_cookie, &mut account_governance_cookie)
        .await
        .unwrap();

    governance_test
        .with_cast_vote(&proposal_cookie, &token_owner_record_cookie, Vote::Yes)
        .await
        .unwrap();

    // Act
    let err = governance_test
        .withdraw_community_tokens(&realm_cookie, &token_owner_record_cookie)
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(
        err,
        GovernanceError::AllVotesMustBeRelinquishedToWithdrawGoverningTokens.into()
    );
}

#[tokio::test]
async fn test_withdraw_governing_tokens_after_relinquishing_vote() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await;

    let mut account_governance_cookie = governance_test
        .with_account_governance(
            &realm_cookie,
            &governed_account_cookie,
            &token_owner_record_cookie,
        )
        .await
        .unwrap();

    let proposal_cookie = governance_test
        .with_signed_off_proposal(&token_owner_record_cookie, &mut account_governance_cookie)
        .await
        .unwrap();

    governance_test
        .with_cast_vote(&proposal_cookie, &token_owner_record_cookie, Vote::Yes)
        .await
        .unwrap();

    governance_test
        .relinquish_vote(&proposal_cookie, &token_owner_record_cookie)
        .await
        .unwrap();

    // Act
    governance_test
        .withdraw_community_tokens(&realm_cookie, &token_owner_record_cookie)
        .await
        .unwrap();

    // Assert
    let source_account = governance_test
        .get_token_account(&token_owner_record_cookie.token_source)
        .await;

    assert_eq!(
        token_owner_record_cookie.token_source_amount,
        source_account.amount
    );
}

#[tokio::test]
async fn test_withdraw_tokens_with_malicious_holding_account_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;
    let realm_cookie = governance_test.with_realm().await;

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await;

    // Try to maliciously withdraw from other token account owned by realm

    let realm_token_account_cookie = governance_test
        .with_token_account(
            &realm_cookie.account.community_mint,
            &realm_cookie.address,
            &realm_cookie.community_mint_authority,
            200,
        )
        .await;

    let mut instruction = withdraw_governing_tokens(
        &governance_test.program_id,
        &realm_cookie.address,
        &token_owner_record_cookie.token_source,
        &token_owner_record_cookie.token_owner.pubkey(),
        &realm_cookie.account.community_mint,
    );

    instruction.accounts[1].pubkey = realm_token_account_cookie.address;

    // Act
    let err = governance_test
        .process_transaction(
            &[instruction],
            Some(&[&token_owner_record_cookie.token_owner]),
        )
        .await
        .err()
        .unwrap();

    // Assert

    assert_eq!(
        err,
        GovernanceError::InvalidGoverningTokenHoldingAccount.into()
    );
}

#[tokio::test]
async fn test_withdraw_governing_tokens_with_outstanding_proposals_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await;
    let mut account_governance_cookie = governance_test
        .with_account_governance(
            &realm_cookie,
            &governed_account_cookie,
            &token_owner_record_cookie,
        )
        .await
        .unwrap();

    governance_test
        .with_signed_off_proposal(&token_owner_record_cookie, &mut account_governance_cookie)
        .await
        .unwrap();

    // Act
    let err = governance_test
        .withdraw_community_tokens(&realm_cookie, &token_owner_record_cookie)
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(
        err,
        GovernanceError::AllProposalsMustBeFinalisedToWithdrawGoverningTokens.into()
    );
}

#[tokio::test]
async fn test_withdraw_governing_tokens_after_proposal_cancelled() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await;
    let mut account_governance_cookie = governance_test
        .with_account_governance(
            &realm_cookie,
            &governed_account_cookie,
            &token_owner_record_cookie,
        )
        .await
        .unwrap();

    let proposal_cookie = governance_test
        .with_signed_off_proposal(&token_owner_record_cookie, &mut account_governance_cookie)
        .await
        .unwrap();

    governance_test
        .cancel_proposal(&proposal_cookie, &token_owner_record_cookie)
        .await
        .unwrap();

    // Act
    governance_test
        .withdraw_community_tokens(&realm_cookie, &token_owner_record_cookie)
        .await
        .unwrap();

    // Assert
    let source_account = governance_test
        .get_token_account(&token_owner_record_cookie.token_source)
        .await;

    assert_eq!(
        token_owner_record_cookie.token_source_amount,
        source_account.amount
    );
}
