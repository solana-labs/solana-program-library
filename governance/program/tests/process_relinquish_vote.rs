#![cfg(feature = "test-sbf")]

mod program_test;

use solana_program::{instruction::AccountMeta, pubkey::Pubkey};
use solana_program_test::tokio;

use program_test::*;
use solana_sdk::signer::Signer;
use spl_governance::{
    error::GovernanceError,
    instruction::{cast_vote, relinquish_vote},
    state::{
        enums::{ProposalState, VoteTipping},
        vote_record::{Vote, VoteChoice},
    },
};

#[tokio::test]
async fn test_relinquish_voted_proposal() {
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
        .with_signed_off_proposal(&token_owner_record_cookie, &mut governance_cookie)
        .await
        .unwrap();

    let mut vote_record_cookie = governance_test
        .with_cast_yes_no_vote(&proposal_cookie, &token_owner_record_cookie, YesNoVote::Yes)
        .await
        .unwrap();

    // Act
    governance_test
        .relinquish_vote(&proposal_cookie, &token_owner_record_cookie)
        .await
        .unwrap();

    // Assert

    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    assert_eq!(100, proposal_account.options[0].vote_weight);
    assert_eq!(ProposalState::Succeeded, proposal_account.state);

    let token_owner_record = governance_test
        .get_token_owner_record_account(&token_owner_record_cookie.address)
        .await;

    assert_eq!(0, token_owner_record.unrelinquished_votes_count);

    let vote_record_account = governance_test
        .get_vote_record_account(&vote_record_cookie.address)
        .await;

    vote_record_cookie.account.is_relinquished = true;
    assert_eq!(vote_record_cookie.account, vote_record_account);
}

#[tokio::test]
async fn test_relinquish_active_yes_vote() {
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

    // Total 300 tokens
    governance_test
        .mint_community_tokens(&realm_cookie, 200)
        .await;

    let proposal_cookie = governance_test
        .with_signed_off_proposal(&token_owner_record_cookie, &mut governance_cookie)
        .await
        .unwrap();

    let vote_record_cookie = governance_test
        .with_cast_yes_no_vote(&proposal_cookie, &token_owner_record_cookie, YesNoVote::Yes)
        .await
        .unwrap();

    // Act
    governance_test
        .relinquish_vote(&proposal_cookie, &token_owner_record_cookie)
        .await
        .unwrap();

    // Assert

    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    assert_eq!(0, proposal_account.options[0].vote_weight);
    assert_eq!(0, proposal_account.deny_vote_weight.unwrap());
    assert_eq!(ProposalState::Voting, proposal_account.state);

    let token_owner_record = governance_test
        .get_token_owner_record_account(&token_owner_record_cookie.address)
        .await;

    assert_eq!(0, token_owner_record.unrelinquished_votes_count);

    let vote_record_account = governance_test
        .bench
        .get_account(&vote_record_cookie.address)
        .await;

    assert_eq!(None, vote_record_account);
}

#[tokio::test]
async fn test_relinquish_active_no_vote() {
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

    // Total 300 tokens
    governance_test
        .mint_community_tokens(&realm_cookie, 200)
        .await;

    let proposal_cookie = governance_test
        .with_signed_off_proposal(&token_owner_record_cookie, &mut governance_cookie)
        .await
        .unwrap();

    let vote_record_cookie = governance_test
        .with_cast_yes_no_vote(&proposal_cookie, &token_owner_record_cookie, YesNoVote::No)
        .await
        .unwrap();

    // Act
    governance_test
        .relinquish_vote(&proposal_cookie, &token_owner_record_cookie)
        .await
        .unwrap();

    // Assert

    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    assert_eq!(0, proposal_account.options[0].vote_weight);
    assert_eq!(0, proposal_account.deny_vote_weight.unwrap());
    assert_eq!(ProposalState::Voting, proposal_account.state);

    let token_owner_record = governance_test
        .get_token_owner_record_account(&token_owner_record_cookie.address)
        .await;

    assert_eq!(0, token_owner_record.unrelinquished_votes_count);

    let vote_record_account = governance_test
        .bench
        .get_account(&vote_record_cookie.address)
        .await;

    assert_eq!(None, vote_record_account);
}

#[tokio::test]
async fn test_relinquish_vote_with_invalid_mint_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    let mut token_owner_record_cookie = governance_test
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
        .with_signed_off_proposal(&token_owner_record_cookie, &mut governance_cookie)
        .await
        .unwrap();

    governance_test
        .with_cast_yes_no_vote(&proposal_cookie, &token_owner_record_cookie, YesNoVote::No)
        .await
        .unwrap();

    token_owner_record_cookie.account.governing_token_mint = Pubkey::new_unique();

    // Act

    let err = governance_test
        .relinquish_vote(&proposal_cookie, &token_owner_record_cookie)
        .await
        .err()
        .unwrap();

    // Assert

    assert_eq!(err, GovernanceError::InvalidGoverningTokenMint.into());
}

#[tokio::test]
async fn test_relinquish_vote_with_governance_authority_must_sign_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    let mut token_owner_record_cookie = governance_test
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

    // Total 300 tokens
    governance_test
        .mint_community_tokens(&realm_cookie, 200)
        .await;

    let proposal_cookie = governance_test
        .with_signed_off_proposal(&token_owner_record_cookie, &mut governance_cookie)
        .await
        .unwrap();

    governance_test
        .with_cast_yes_no_vote(&proposal_cookie, &token_owner_record_cookie, YesNoVote::No)
        .await
        .unwrap();

    // Try to use a different owner to sign
    let token_owner_record_cookie2 = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    token_owner_record_cookie.token_owner = token_owner_record_cookie2.token_owner;

    // Act

    let err = governance_test
        .relinquish_vote(&proposal_cookie, &token_owner_record_cookie)
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
async fn test_relinquish_vote_with_invalid_vote_record_error() {
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

    let token_owner_record_cookie2 = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    // Total 400 tokens
    governance_test
        .mint_community_tokens(&realm_cookie, 200)
        .await;

    let proposal_cookie = governance_test
        .with_signed_off_proposal(&token_owner_record_cookie, &mut governance_cookie)
        .await
        .unwrap();

    governance_test
        .with_cast_yes_no_vote(&proposal_cookie, &token_owner_record_cookie, YesNoVote::No)
        .await
        .unwrap();

    let vote_record_cookie2 = governance_test
        .with_cast_yes_no_vote(
            &proposal_cookie,
            &token_owner_record_cookie2,
            YesNoVote::Yes,
        )
        .await
        .unwrap();

    // // Act

    let err = governance_test
        .relinquish_vote_using_instruction(&proposal_cookie, &token_owner_record_cookie, |i| {
            i.accounts[4] = AccountMeta::new(vote_record_cookie2.address, false)
            // Try to use a vote_record for other token owner
        })
        .await
        .err()
        .unwrap();

    // // Assert

    assert_eq!(
        err,
        GovernanceError::InvalidGoverningTokenOwnerForVoteRecord.into()
    );
}

#[tokio::test]
async fn test_relinquish_vote_with_already_relinquished_error() {
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
        .with_signed_off_proposal(&token_owner_record_cookie, &mut governance_cookie)
        .await
        .unwrap();

    let vote_record_cookie = governance_test
        .with_cast_yes_no_vote(&proposal_cookie, &token_owner_record_cookie, YesNoVote::No)
        .await
        .unwrap();

    governance_test
        .relinquish_vote(&proposal_cookie, &token_owner_record_cookie)
        .await
        .unwrap();

    // Ensure vote is relinquished
    let vote_record_account = governance_test
        .get_vote_record_account(&vote_record_cookie.address)
        .await;

    assert!(vote_record_account.is_relinquished);

    governance_test
        .mint_community_tokens(&realm_cookie, 10)
        .await;

    governance_test.advance_clock().await;
    // Act

    let err = governance_test
        .relinquish_vote(&proposal_cookie, &token_owner_record_cookie)
        .await
        .err()
        .unwrap();

    // Assert

    assert_eq!(err, GovernanceError::VoteAlreadyRelinquished.into());
}

#[tokio::test]
async fn test_relinquish_proposal_with_cannot_relinquish_in_finalizing_state_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    // Deposit 100 tokens
    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    // Add 200 tokens (total 300) to prevent the vote being tipped
    governance_test
        .mint_community_tokens(&realm_cookie, 200)
        .await;

    let mut governance_cookie = governance_test
        .with_governance(
            &realm_cookie,
            &governed_account_cookie,
            &token_owner_record_cookie,
        )
        .await
        .unwrap();

    let proposal_cookie = governance_test
        .with_signed_off_proposal(&token_owner_record_cookie, &mut governance_cookie)
        .await
        .unwrap();

    let clock = governance_test.bench.get_clock().await;

    governance_test
        .with_cast_yes_no_vote(&proposal_cookie, &token_owner_record_cookie, YesNoVote::Yes)
        .await
        .unwrap();

    // Advance timestamp past max_voting_time
    governance_test
        .advance_clock_past_timestamp(
            governance_cookie.account.config.voting_base_time as i64 + clock.unix_timestamp,
        )
        .await;

    // Act
    let err = governance_test
        .relinquish_vote(&proposal_cookie, &token_owner_record_cookie)
        .await
        .err()
        .unwrap();

    // Assert

    assert_eq!(
        err,
        GovernanceError::CannotRelinquishInFinalizingState.into()
    );
}

#[tokio::test]
async fn test_relinquish_and_cast_vote_in_single_transaction() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let mut governance_config = governance_test.get_default_governance_config();
    governance_config.community_vote_tipping = VoteTipping::Disabled;

    let mut governance_cookie = governance_test
        .with_governance_using_config(
            &realm_cookie,
            &governed_account_cookie,
            &token_owner_record_cookie,
            &governance_config,
        )
        .await
        .unwrap();

    let proposal_cookie = governance_test
        .with_signed_off_proposal(&token_owner_record_cookie, &mut governance_cookie)
        .await
        .unwrap();

    let vote_record_cookie = governance_test
        .with_cast_yes_no_vote(&proposal_cookie, &token_owner_record_cookie, YesNoVote::Yes)
        .await
        .unwrap();

    let relinquish_vote_ix = relinquish_vote(
        &governance_test.program_id,
        &token_owner_record_cookie.account.realm,
        &proposal_cookie.account.governance,
        &proposal_cookie.address,
        &token_owner_record_cookie.address,
        &token_owner_record_cookie.account.governing_token_mint,
        Some(token_owner_record_cookie.token_owner.pubkey()),
        Some(governance_test.bench.payer.pubkey()),
    );

    let cast_vote_ix = cast_vote(
        &governance_test.program_id,
        &token_owner_record_cookie.account.realm,
        &proposal_cookie.account.governance,
        &proposal_cookie.address,
        &proposal_cookie.account.token_owner_record,
        &token_owner_record_cookie.address,
        &token_owner_record_cookie.token_owner.pubkey(),
        &token_owner_record_cookie.account.governing_token_mint,
        &governance_test.bench.payer.pubkey(),
        None,
        None,
        Vote::Approve(vec![VoteChoice {
            rank: 0,
            weight_percentage: 100,
        }]),
    );

    // Act
    governance_test
        .bench
        .process_transaction(
            &[relinquish_vote_ix, cast_vote_ix],
            Some(&[&token_owner_record_cookie.token_owner]),
        )
        .await
        .unwrap();

    // Assert
    let vote_record_account = governance_test
        .get_vote_record_account(&vote_record_cookie.address)
        .await;

    assert_eq!(vote_record_cookie.account, vote_record_account);
}

#[tokio::test]
async fn test_change_yes_vote_to_no_within_cool_off_time() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    // Set none default voting cool off time
    let mut governance_config = governance_test.get_default_governance_config();
    governance_config.voting_cool_off_time = 50;

    let mut governance_cookie = governance_test
        .with_governance_using_config(
            &realm_cookie,
            &governed_account_cookie,
            &token_owner_record_cookie,
            &governance_config,
        )
        .await
        .unwrap();

    // Total 300 tokens
    governance_test
        .mint_community_tokens(&realm_cookie, 200)
        .await;

    let proposal_cookie = governance_test
        .with_signed_off_proposal(&token_owner_record_cookie, &mut governance_cookie)
        .await
        .unwrap();

    governance_test
        .with_cast_yes_no_vote(&proposal_cookie, &token_owner_record_cookie, YesNoVote::Yes)
        .await
        .unwrap();

    // Advance timestamp into voting_cool_off_time
    let clock = governance_test.bench.get_clock().await;

    governance_test
        .advance_clock_past_timestamp(
            clock.unix_timestamp + governance_cookie.account.config.voting_base_time as i64,
        )
        .await;

    // Act
    governance_test
        .relinquish_vote(&proposal_cookie, &token_owner_record_cookie)
        .await
        .unwrap();

    governance_test
        .with_cast_yes_no_vote(&proposal_cookie, &token_owner_record_cookie, YesNoVote::No)
        .await
        .unwrap();

    // Assert

    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    assert_eq!(0, proposal_account.options[0].vote_weight);
    assert_eq!(100, proposal_account.deny_vote_weight.unwrap());
    assert_eq!(ProposalState::Voting, proposal_account.state);
}
