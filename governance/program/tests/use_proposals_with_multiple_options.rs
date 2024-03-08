#![cfg(feature = "test-sbf")]

use solana_program_test::*;

mod program_test;

use {
    program_test::*,
    spl_governance::{
        error::GovernanceError,
        state::{
            enums::{ProposalState, VoteThreshold},
            proposal::{MultiChoiceType, OptionVoteResult, VoteType},
            vote_record::{Vote, VoteChoice},
        },
    },
};

#[tokio::test]
async fn test_create_proposal_with_single_choice_options_and_deny_option() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let mut governance_cookie = governance_test
        .with_governance(&realm_cookie, &token_owner_record_cookie)
        .await
        .unwrap();

    let options = vec!["option 1".to_string(), "option 2".to_string()];

    // Act
    let proposal_cookie = governance_test
        .with_multi_option_proposal(
            &token_owner_record_cookie,
            &mut governance_cookie,
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
    assert!(proposal_account.deny_vote_weight.is_some());

    assert_eq!(proposal_cookie.account, proposal_account);
}

#[tokio::test]
async fn test_create_proposal_with_multiple_choice_options_and_without_deny_option() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let mut governance_cookie = governance_test
        .with_governance(&realm_cookie, &token_owner_record_cookie)
        .await
        .unwrap();

    let options = vec!["option 1".to_string(), "option 2".to_string()];

    // Act
    let proposal_cookie = governance_test
        .with_multi_option_proposal(
            &token_owner_record_cookie,
            &mut governance_cookie,
            options,
            false,
            VoteType::MultiChoice {
                choice_type: MultiChoiceType::FullWeight,
                min_voter_options: 1,
                max_winning_options: 2,
                max_voter_options: 2,
            },
        )
        .await
        .unwrap();

    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    assert_eq!(
        proposal_account.vote_type,
        VoteType::MultiChoice {
            choice_type: MultiChoiceType::FullWeight,
            min_voter_options: 1,
            max_winning_options: 2,
            max_voter_options: 2,
        }
    );
    assert!(proposal_account.deny_vote_weight.is_none());

    assert_eq!(proposal_cookie.account, proposal_account);
}

#[tokio::test]
async fn test_insert_transaction_with_proposal_not_executable_error() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let mut governance_cookie = governance_test
        .with_governance(&realm_cookie, &token_owner_record_cookie)
        .await
        .unwrap();

    let mut proposal_cookie = governance_test
        .with_multi_option_proposal(
            &token_owner_record_cookie,
            &mut governance_cookie,
            vec!["option 1".to_string(), "option 2".to_string()],
            false,
            VoteType::SingleChoice,
        )
        .await
        .unwrap();

    // Act
    let err = governance_test
        .with_nop_transaction(&mut proposal_cookie, &token_owner_record_cookie, 0, None)
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(err, GovernanceError::ProposalIsNotExecutable.into());
}

#[tokio::test]
async fn test_insert_transactions_for_multiple_options() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let mut governance_cookie = governance_test
        .with_governance(&realm_cookie, &token_owner_record_cookie)
        .await
        .unwrap();

    let mut proposal_cookie = governance_test
        .with_multi_option_proposal(
            &token_owner_record_cookie,
            &mut governance_cookie,
            vec!["option 1".to_string(), "option 2".to_string()],
            true,
            VoteType::SingleChoice,
        )
        .await
        .unwrap();

    // Act

    // option 1 / transaction 0
    governance_test
        .with_nop_transaction(&mut proposal_cookie, &token_owner_record_cookie, 1, Some(0))
        .await
        .unwrap();

    // option 1 / transaction 1
    governance_test
        .with_nop_transaction(&mut proposal_cookie, &token_owner_record_cookie, 1, Some(1))
        .await
        .unwrap();

    // option 1 / transaction 2
    governance_test
        .with_nop_transaction(&mut proposal_cookie, &token_owner_record_cookie, 1, Some(2))
        .await
        .unwrap();

    // option 0 / transaction 0
    governance_test
        .with_nop_transaction(&mut proposal_cookie, &token_owner_record_cookie, 0, Some(0))
        .await
        .unwrap();

    // option 0 / transaction 1
    governance_test
        .with_nop_transaction(&mut proposal_cookie, &token_owner_record_cookie, 0, Some(1))
        .await
        .unwrap();

    // Assert
    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    assert_eq!(2, proposal_account.options[0].transactions_count);
    assert_eq!(3, proposal_account.options[1].transactions_count);
}

#[tokio::test]
async fn test_vote_on_none_executable_single_choice_proposal_with_multiple_options() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let mut governance_cookie = governance_test
        .with_governance(&realm_cookie, &token_owner_record_cookie)
        .await
        .unwrap();

    let proposal_cookie = governance_test
        .with_multi_option_proposal(
            &token_owner_record_cookie,
            &mut governance_cookie,
            vec!["option 1".to_string(), "option 2".to_string()],
            false,
            VoteType::SingleChoice,
        )
        .await
        .unwrap();

    let signatory_record_cookie = governance_test
        .with_signatory(
            &proposal_cookie,
            &governance_cookie,
            &token_owner_record_cookie,
        )
        .await
        .unwrap();

    let clock = governance_test.bench.get_clock().await;

    governance_test
        .sign_off_proposal(&proposal_cookie, &signatory_record_cookie)
        .await
        .unwrap();

    let vote = Vote::Approve(vec![
        VoteChoice {
            rank: 0,
            weight_percentage: 100,
        },
        VoteChoice {
            rank: 0,
            weight_percentage: 0,
        },
    ]);

    // Act
    governance_test
        .with_cast_vote(&proposal_cookie, &token_owner_record_cookie, vote)
        .await
        .unwrap();

    // Advance timestamp past voting_base_time
    governance_test
        .advance_clock_past_timestamp(
            governance_cookie.account.config.voting_base_time as i64 + clock.unix_timestamp,
        )
        .await;

    governance_test
        .finalize_vote(&realm_cookie, &proposal_cookie, None)
        .await
        .unwrap();

    // Assert

    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    assert_eq!(
        OptionVoteResult::Succeeded,
        proposal_account.options[0].vote_result
    );

    assert_eq!(
        OptionVoteResult::Defeated,
        proposal_account.options[1].vote_result
    );

    // None executable proposal transitions to Completed when vote is finalized
    assert_eq!(ProposalState::Completed, proposal_account.state);
}

#[tokio::test]
async fn test_vote_on_none_executable_multi_choice_proposal_with_multiple_options() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let mut governance_cookie = governance_test
        .with_governance(&realm_cookie, &token_owner_record_cookie)
        .await
        .unwrap();

    let proposal_cookie = governance_test
        .with_multi_option_proposal(
            &token_owner_record_cookie,
            &mut governance_cookie,
            vec![
                "option 1".to_string(),
                "option 2".to_string(),
                "option 3".to_string(),
            ],
            false,
            VoteType::MultiChoice {
                choice_type: MultiChoiceType::FullWeight,
                min_voter_options: 1,
                max_winning_options: 3,
                max_voter_options: 3,
            },
        )
        .await
        .unwrap();

    let signatory_record_cookie = governance_test
        .with_signatory(
            &proposal_cookie,
            &governance_cookie,
            &token_owner_record_cookie,
        )
        .await
        .unwrap();

    let clock = governance_test.bench.get_clock().await;

    governance_test
        .sign_off_proposal(&proposal_cookie, &signatory_record_cookie)
        .await
        .unwrap();

    let vote = Vote::Approve(vec![
        VoteChoice {
            rank: 0,
            weight_percentage: 100,
        },
        VoteChoice {
            rank: 0,
            weight_percentage: 100,
        },
        VoteChoice {
            rank: 0,
            weight_percentage: 0,
        },
    ]);

    // Act
    governance_test
        .with_cast_vote(&proposal_cookie, &token_owner_record_cookie, vote)
        .await
        .unwrap();

    // Advance timestamp past voting_base_time
    governance_test
        .advance_clock_past_timestamp(
            governance_cookie.account.config.voting_base_time as i64 + clock.unix_timestamp,
        )
        .await;

    governance_test
        .finalize_vote(&realm_cookie, &proposal_cookie, None)
        .await
        .unwrap();

    // Assert

    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    assert_eq!(
        OptionVoteResult::Succeeded,
        proposal_account.options[0].vote_result
    );

    assert_eq!(
        OptionVoteResult::Succeeded,
        proposal_account.options[1].vote_result
    );

    assert_eq!(
        OptionVoteResult::Defeated,
        proposal_account.options[2].vote_result
    );

    // None executable proposal transitions to Completed when vote is finalized
    assert_eq!(ProposalState::Completed, proposal_account.state);
}

#[tokio::test]
async fn test_vote_on_executable_proposal_with_multiple_options_and_partial_success() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;

    // 100 tokens
    let token_owner_record_cookie1 = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    // 100 tokens
    let token_owner_record_cookie2 = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    // 100 tokens
    let token_owner_record_cookie3 = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    // 100 tokes approval quorum
    let mut governance_config = governance_test.get_default_governance_config();
    governance_config.community_vote_threshold = VoteThreshold::YesVotePercentage(30);

    let mut governance_cookie = governance_test
        .with_governance_using_config(
            &realm_cookie,
            &token_owner_record_cookie1,
            &governance_config,
        )
        .await
        .unwrap();

    let proposal_cookie = governance_test
        .with_multi_option_proposal(
            &token_owner_record_cookie1,
            &mut governance_cookie,
            vec![
                "option 1".to_string(),
                "option 2".to_string(),
                "option 3".to_string(),
            ],
            true,
            VoteType::MultiChoice {
                choice_type: MultiChoiceType::FullWeight,
                min_voter_options: 1,
                max_winning_options: 3,
                max_voter_options: 3,
            },
        )
        .await
        .unwrap();

    let signatory_record_cookie = governance_test
        .with_signatory(
            &proposal_cookie,
            &governance_cookie,
            &token_owner_record_cookie1,
        )
        .await
        .unwrap();

    let clock = governance_test.bench.get_clock().await;

    governance_test
        .sign_off_proposal(&proposal_cookie, &signatory_record_cookie)
        .await
        .unwrap();

    // Act

    // choice 1: 200
    // choice 2: 100
    // choice 3: 0
    // deny: 100
    // yes threshold: 100

    let vote1 = Vote::Approve(vec![
        VoteChoice {
            rank: 0,
            weight_percentage: 100,
        },
        VoteChoice {
            rank: 0,
            weight_percentage: 100,
        },
        VoteChoice {
            rank: 0,
            weight_percentage: 0,
        },
    ]);

    governance_test
        .with_cast_vote(&proposal_cookie, &token_owner_record_cookie1, vote1)
        .await
        .unwrap();

    let vote2 = Vote::Approve(vec![
        VoteChoice {
            rank: 0,
            weight_percentage: 100,
        },
        VoteChoice {
            rank: 0,
            weight_percentage: 0,
        },
        VoteChoice {
            rank: 0,
            weight_percentage: 0,
        },
    ]);

    governance_test
        .with_cast_vote(&proposal_cookie, &token_owner_record_cookie2, vote2)
        .await
        .unwrap();

    governance_test
        .with_cast_vote(&proposal_cookie, &token_owner_record_cookie3, Vote::Deny)
        .await
        .unwrap();

    // Advance timestamp past voting_base_time
    governance_test
        .advance_clock_past_timestamp(
            governance_cookie.account.config.voting_base_time as i64 + clock.unix_timestamp,
        )
        .await;

    governance_test
        .finalize_vote(&realm_cookie, &proposal_cookie, None)
        .await
        .unwrap();

    // Assert

    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    assert_eq!(200, proposal_account.options[0].vote_weight);

    assert_eq!(
        OptionVoteResult::Succeeded,
        proposal_account.options[0].vote_result
    );

    assert_eq!(100, proposal_account.options[1].vote_weight);
    assert_eq!(
        OptionVoteResult::Defeated,
        proposal_account.options[1].vote_result
    );

    assert_eq!(0, proposal_account.options[2].vote_weight);
    assert_eq!(
        OptionVoteResult::Defeated,
        proposal_account.options[2].vote_result
    );
}

#[tokio::test]
async fn test_execute_proposal_with_multiple_options_and_partial_success() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;

    // 100 tokens
    let token_owner_record_cookie1 = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    // 100 tokens
    let token_owner_record_cookie2 = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    // 100 tokens
    let token_owner_record_cookie3 = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    // 100 tokes approval quorum
    let mut governance_config = governance_test.get_default_governance_config();
    governance_config.community_vote_threshold = VoteThreshold::YesVotePercentage(30);

    let mut governance_cookie = governance_test
        .with_governance_using_config(
            &realm_cookie,
            &token_owner_record_cookie1,
            &governance_config,
        )
        .await
        .unwrap();

    let governed_mint_cookie = governance_test.with_governed_mint(&governance_cookie).await;

    let mut proposal_cookie = governance_test
        .with_multi_option_proposal(
            &token_owner_record_cookie1,
            &mut governance_cookie,
            vec![
                "option 1".to_string(),
                "option 2".to_string(),
                "option 3".to_string(),
            ],
            true,
            VoteType::MultiChoice {
                choice_type: MultiChoiceType::FullWeight,
                min_voter_options: 1,
                max_winning_options: 3,
                max_voter_options: 3,
            },
        )
        .await
        .unwrap();

    let proposal_transaction_cookie1 = governance_test
        .with_mint_tokens_transaction(
            &governed_mint_cookie,
            &mut proposal_cookie,
            &token_owner_record_cookie1,
            0,
            Some(0),
        )
        .await
        .unwrap();

    let proposal_transaction_cookie2 = governance_test
        .with_mint_tokens_transaction(
            &governed_mint_cookie,
            &mut proposal_cookie,
            &token_owner_record_cookie1,
            1,
            Some(0),
        )
        .await
        .unwrap();

    let proposal_transaction_cookie3 = governance_test
        .with_mint_tokens_transaction(
            &governed_mint_cookie,
            &mut proposal_cookie,
            &token_owner_record_cookie1,
            2,
            Some(0),
        )
        .await
        .unwrap();

    let signatory_record_cookie = governance_test
        .with_signatory(
            &proposal_cookie,
            &governance_cookie,
            &token_owner_record_cookie1,
        )
        .await
        .unwrap();

    governance_test
        .sign_off_proposal(&proposal_cookie, &signatory_record_cookie)
        .await
        .unwrap();

    // deny: 100
    // choice 1: 100 -> Defeated
    // choice 2: 200 -> Success
    // choice 3: 0 -> Defeated
    // yes threshold: 100

    governance_test
        .with_cast_vote(&proposal_cookie, &token_owner_record_cookie3, Vote::Deny)
        .await
        .unwrap();

    let vote1 = Vote::Approve(vec![
        VoteChoice {
            rank: 0,
            weight_percentage: 0,
        },
        VoteChoice {
            rank: 0,
            weight_percentage: 100,
        },
        VoteChoice {
            rank: 0,
            weight_percentage: 0,
        },
    ]);

    governance_test
        .with_cast_vote(&proposal_cookie, &token_owner_record_cookie1, vote1)
        .await
        .unwrap();

    let vote2 = Vote::Approve(vec![
        VoteChoice {
            rank: 0,
            weight_percentage: 100,
        },
        VoteChoice {
            rank: 0,
            weight_percentage: 100,
        },
        VoteChoice {
            rank: 0,
            weight_percentage: 0,
        },
    ]);

    governance_test
        .with_cast_vote(&proposal_cookie, &token_owner_record_cookie2, vote2)
        .await
        .unwrap();

    // Advance timestamp past voting_base_time
    governance_test
        .advance_clock_by_min_timespan(governance_cookie.account.config.voting_base_time as u64)
        .await;

    governance_test
        .finalize_vote(&realm_cookie, &proposal_cookie, None)
        .await
        .unwrap();

    // Advance timestamp past hold_up_time
    governance_test
        .advance_clock_by_min_timespan(
            governance_cookie.account.config.transactions_hold_up_time as u64,
        )
        .await;

    let mut proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    assert_eq!(ProposalState::Succeeded, proposal_account.state);

    // Act

    let transaction1_err = governance_test
        .execute_proposal_transaction(&proposal_cookie, &proposal_transaction_cookie1)
        .await
        .err()
        .unwrap();

    governance_test
        .execute_proposal_transaction(&proposal_cookie, &proposal_transaction_cookie2)
        .await
        .unwrap();

    let transaction3_err = governance_test
        .execute_proposal_transaction(&proposal_cookie, &proposal_transaction_cookie3)
        .await
        .err()
        .unwrap();

    // Assert
    proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    assert_eq!(ProposalState::Completed, proposal_account.state);

    assert_eq!(
        transaction1_err,
        GovernanceError::CannotExecuteDefeatedOption.into()
    );

    assert_eq!(
        transaction3_err,
        GovernanceError::InvalidStateCannotExecuteTransaction.into()
    );
}

#[tokio::test]
async fn test_try_execute_proposal_with_multiple_options_and_full_deny() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;

    // 100 tokens
    let token_owner_record_cookie1 = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    // 100 tokens
    let token_owner_record_cookie2 = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    // 100 tokes approval quorum
    let mut governance_config = governance_test.get_default_governance_config();
    governance_config.community_vote_threshold = VoteThreshold::YesVotePercentage(30);

    let mut governance_cookie = governance_test
        .with_governance_using_config(
            &realm_cookie,
            &token_owner_record_cookie1,
            &governance_config,
        )
        .await
        .unwrap();

    let governed_mint_cookie = governance_test.with_governed_mint(&governance_cookie).await;

    let mut proposal_cookie = governance_test
        .with_multi_option_proposal(
            &token_owner_record_cookie1,
            &mut governance_cookie,
            vec![
                "option 1".to_string(),
                "option 2".to_string(),
                "option 3".to_string(),
            ],
            true,
            VoteType::MultiChoice {
                choice_type: MultiChoiceType::FullWeight,
                min_voter_options: 1,
                max_winning_options: 3,
                max_voter_options: 3,
            },
        )
        .await
        .unwrap();

    let proposal_transaction_cookie1 = governance_test
        .with_mint_tokens_transaction(
            &governed_mint_cookie,
            &mut proposal_cookie,
            &token_owner_record_cookie1,
            0,
            Some(0),
        )
        .await
        .unwrap();

    let proposal_transaction_cookie2 = governance_test
        .with_mint_tokens_transaction(
            &governed_mint_cookie,
            &mut proposal_cookie,
            &token_owner_record_cookie1,
            1,
            Some(0),
        )
        .await
        .unwrap();

    let proposal_transaction_cookie3 = governance_test
        .with_mint_tokens_transaction(
            &governed_mint_cookie,
            &mut proposal_cookie,
            &token_owner_record_cookie1,
            2,
            Some(0),
        )
        .await
        .unwrap();

    let signatory_record_cookie = governance_test
        .with_signatory(
            &proposal_cookie,
            &governance_cookie,
            &token_owner_record_cookie1,
        )
        .await
        .unwrap();

    governance_test
        .sign_off_proposal(&proposal_cookie, &signatory_record_cookie)
        .await
        .unwrap();

    governance_test
        .with_cast_vote(&proposal_cookie, &token_owner_record_cookie1, Vote::Deny)
        .await
        .unwrap();

    governance_test
        .with_cast_vote(&proposal_cookie, &token_owner_record_cookie2, Vote::Deny)
        .await
        .unwrap();

    // Advance timestamp past voting_base_time
    governance_test
        .advance_clock_by_min_timespan(governance_cookie.account.config.voting_base_time as u64)
        .await;

    governance_test
        .finalize_vote(&realm_cookie, &proposal_cookie, None)
        .await
        .unwrap();

    // Advance timestamp past hold_up_time
    governance_test
        .advance_clock_by_min_timespan(
            governance_cookie.account.config.transactions_hold_up_time as u64,
        )
        .await;

    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    assert_eq!(ProposalState::Defeated, proposal_account.state);

    // Act

    let mut err = governance_test
        .execute_proposal_transaction(&proposal_cookie, &proposal_transaction_cookie1)
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(
        err,
        GovernanceError::InvalidStateCannotExecuteTransaction.into()
    );

    // Act

    err = governance_test
        .execute_proposal_transaction(&proposal_cookie, &proposal_transaction_cookie2)
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(
        err,
        GovernanceError::InvalidStateCannotExecuteTransaction.into()
    );

    // Act

    err = governance_test
        .execute_proposal_transaction(&proposal_cookie, &proposal_transaction_cookie3)
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(
        err,
        GovernanceError::InvalidStateCannotExecuteTransaction.into()
    );
}

#[tokio::test]
async fn test_create_proposal_with_10_options_and_cast_vote() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let mut governance_cookie = governance_test
        .with_governance(&realm_cookie, &token_owner_record_cookie)
        .await
        .unwrap();

    let options_count = 10;

    let options: Vec<String> = (0..options_count)
        .map(|n| format!("option {:?}", n))
        .collect();

    let options_len = options.len() as u8;

    let proposal_cookie = governance_test
        .with_multi_option_proposal(
            &token_owner_record_cookie,
            &mut governance_cookie,
            options,
            false,
            VoteType::MultiChoice {
                choice_type: MultiChoiceType::FullWeight,
                min_voter_options: 1,
                max_winning_options: options_len,
                max_voter_options: options_len,
            },
        )
        .await
        .unwrap();

    governance_test
        .sign_off_proposal_by_owner(&proposal_cookie, &token_owner_record_cookie)
        .await
        .unwrap();

    let vote = Vote::Approve(
        (0..options_count)
            .map(|_| VoteChoice {
                rank: 0,
                weight_percentage: 100,
            })
            .collect(),
    );

    // Act
    governance_test
        .with_cast_vote(&proposal_cookie, &token_owner_record_cookie, vote)
        .await
        .unwrap();

    let clock = governance_test.bench.get_clock().await;

    governance_test
        .advance_clock_past_timestamp(
            governance_cookie.account.config.voting_base_time as i64 + clock.unix_timestamp,
        )
        .await;

    governance_test
        .finalize_vote(&realm_cookie, &proposal_cookie, None)
        .await
        .unwrap();

    // Assert

    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    assert_eq!(
        proposal_account.vote_type,
        VoteType::MultiChoice {
            choice_type: MultiChoiceType::FullWeight,
            min_voter_options: 1,
            max_winning_options: options_len,
            max_voter_options: options_len,
        }
    );
    assert!(proposal_account.deny_vote_weight.is_none());

    assert_eq!(ProposalState::Completed, proposal_account.state);
}

#[tokio::test]
async fn test_vote_multi_weighted_choice_proposal_non_executable() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let mut governance_config = governance_test.get_default_governance_config();
    governance_config.community_vote_threshold = VoteThreshold::YesVotePercentage(30);

    let mut governance_cookie = governance_test
        .with_governance_using_config(
            &realm_cookie,
            &token_owner_record_cookie,
            &governance_config,
        )
        .await
        .unwrap();

    let proposal_cookie = governance_test
        .with_multi_option_proposal(
            &token_owner_record_cookie,
            &mut governance_cookie,
            vec![
                "option 1".to_string(),
                "option 2".to_string(),
                "option 3".to_string(),
                "option 4".to_string(),
            ],
            false,
            VoteType::MultiChoice {
                choice_type: MultiChoiceType::Weighted,
                min_voter_options: 1,
                max_winning_options: 4,
                max_voter_options: 4,
            },
        )
        .await
        .unwrap();

    let clock = governance_test.bench.get_clock().await;

    governance_test
        .sign_off_proposal_by_owner(&proposal_cookie, &token_owner_record_cookie)
        .await
        .unwrap();

    let vote = Vote::Approve(vec![
        VoteChoice {
            rank: 0,
            weight_percentage: 30,
        },
        VoteChoice {
            rank: 0,
            weight_percentage: 29,
        },
        VoteChoice {
            rank: 0,
            weight_percentage: 41,
        },
        VoteChoice {
            rank: 0,
            weight_percentage: 0,
        },
    ]);

    // Act
    governance_test
        .with_cast_vote(&proposal_cookie, &token_owner_record_cookie, vote)
        .await
        .unwrap();

    governance_test
        .advance_clock_past_timestamp(
            governance_cookie.account.config.voting_base_time as i64 + clock.unix_timestamp,
        )
        .await;

    governance_test
        .finalize_vote(&realm_cookie, &proposal_cookie, None)
        .await
        .unwrap();

    // Assert
    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    assert_eq!(
        OptionVoteResult::Succeeded,
        proposal_account.options[0].vote_result
    );
    assert_eq!(
        OptionVoteResult::Defeated,
        proposal_account.options[1].vote_result
    );
    assert_eq!(
        OptionVoteResult::Succeeded,
        proposal_account.options[2].vote_result
    );
    assert_eq!(
        OptionVoteResult::Defeated,
        proposal_account.options[3].vote_result
    );
    assert_eq!(
        (token_owner_record_cookie.token_source_amount as f32 * 0.3) as u64,
        proposal_account.options[0].vote_weight
    );
    assert_eq!(
        (token_owner_record_cookie.token_source_amount as f32 * 0.29) as u64,
        proposal_account.options[1].vote_weight
    );
    assert_eq!(
        (token_owner_record_cookie.token_source_amount as f32 * 0.41) as u64,
        proposal_account.options[2].vote_weight
    );
    assert_eq!(0_u64, proposal_account.options[3].vote_weight);

    // None executable proposal transitions to Completed when vote is finalized
    assert_eq!(ProposalState::Completed, proposal_account.state);
}

#[tokio::test]
async fn test_vote_multi_weighted_choice_proposal_with_partial_success() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;

    // 100 tokens each, sum 300 tokens
    let token_owner_record_cookie1 = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();
    let token_owner_record_cookie2 = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();
    let token_owner_record_cookie3 = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    // 60 tokes approval quorum as 20% of 300 is 60
    let mut governance_config = governance_test.get_default_governance_config();
    governance_config.community_vote_threshold = VoteThreshold::YesVotePercentage(20);

    let mut governance_cookie = governance_test
        .with_governance_using_config(
            &realm_cookie,
            &token_owner_record_cookie1,
            &governance_config,
        )
        .await
        .unwrap();

    let governed_mint_cookie = governance_test.with_governed_mint(&governance_cookie).await;

    let mut proposal_cookie = governance_test
        .with_multi_option_proposal(
            &token_owner_record_cookie1,
            &mut governance_cookie,
            vec![
                "option 1".to_string(),
                "option 2".to_string(),
                "option 3".to_string(),
                "option 4".to_string(),
            ],
            true,
            VoteType::MultiChoice {
                choice_type: MultiChoiceType::Weighted,
                min_voter_options: 1,
                max_winning_options: 4,
                max_voter_options: 4,
            },
        )
        .await
        .unwrap();

    let proposal_transaction_cookie1 = governance_test
        .with_mint_tokens_transaction(
            &governed_mint_cookie,
            &mut proposal_cookie,
            &token_owner_record_cookie1,
            0,
            Some(0),
        )
        .await
        .unwrap();
    let proposal_transaction_cookie2 = governance_test
        .with_mint_tokens_transaction(
            &governed_mint_cookie,
            &mut proposal_cookie,
            &token_owner_record_cookie1,
            1,
            Some(0),
        )
        .await
        .unwrap();
    let proposal_transaction_cookie3 = governance_test
        .with_mint_tokens_transaction(
            &governed_mint_cookie,
            &mut proposal_cookie,
            &token_owner_record_cookie1,
            2,
            Some(0),
        )
        .await
        .unwrap();
    let proposal_transaction_cookie4 = governance_test
        .with_mint_tokens_transaction(
            &governed_mint_cookie,
            &mut proposal_cookie,
            &token_owner_record_cookie1,
            3,
            Some(0),
        )
        .await
        .unwrap();

    governance_test
        .sign_off_proposal_by_owner(&proposal_cookie, &token_owner_record_cookie1)
        .await
        .unwrap();

    // vote1:
    //   deny: 100
    // vote2 + vote3:
    //   choice 1: 0 -> Defeated
    //   choice 2: 91 -> Defeated (91 is over 60, 20% from 300, but deny overrules)
    //   choice 3: 101 -> Success
    //   choice 4: 8 -> Defeated (below of 60)

    let vote1 = Vote::Approve(vec![
        VoteChoice {
            rank: 0,
            weight_percentage: 0,
        },
        VoteChoice {
            rank: 0,
            weight_percentage: 30,
        },
        VoteChoice {
            rank: 0,
            weight_percentage: 70,
        },
        VoteChoice {
            rank: 0,
            weight_percentage: 0,
        },
    ]);
    governance_test
        .with_cast_vote(&proposal_cookie, &token_owner_record_cookie1, vote1)
        .await
        .expect("Voting the vote 1 of owner 1 should succeed");

    let vote2 = Vote::Approve(vec![
        VoteChoice {
            rank: 0,
            weight_percentage: 0,
        },
        VoteChoice {
            rank: 0,
            weight_percentage: 61,
        },
        VoteChoice {
            rank: 0,
            weight_percentage: 31,
        },
        VoteChoice {
            rank: 0,
            weight_percentage: 8,
        },
    ]);
    governance_test
        .with_cast_vote(&proposal_cookie, &token_owner_record_cookie2, vote2)
        .await
        .expect("Voting the vote 1 of owner 1 should succeed");

    governance_test
        .with_cast_vote(&proposal_cookie, &token_owner_record_cookie3, Vote::Deny)
        .await
        .expect("Casting deny vote of owner 3 should succeed");

    let clock = governance_test.bench.get_clock().await;
    governance_test
        .advance_clock_past_timestamp(
            governance_cookie.account.config.voting_base_time as i64 + clock.unix_timestamp,
        )
        .await;
    governance_test
        .finalize_vote(&realm_cookie, &proposal_cookie, None)
        .await
        .unwrap();
    // Advance timestamp past hold_up_time
    governance_test
        .advance_clock_by_min_timespan(
            governance_cookie.account.config.transactions_hold_up_time as u64,
        )
        .await;

    let mut proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    assert_eq!(ProposalState::Succeeded, proposal_account.state);

    // Act
    let transaction1_err = governance_test
        .execute_proposal_transaction(&proposal_cookie, &proposal_transaction_cookie1)
        .await
        .expect_err("Choice 1 should fail to execute, it hasn't got enough votes");
    let transaction2_err = governance_test
        .execute_proposal_transaction(&proposal_cookie, &proposal_transaction_cookie2)
        .await
        .expect_err("Choice 2 should fail to execute, it hasn't got enough votes");
    governance_test
        .execute_proposal_transaction(&proposal_cookie, &proposal_transaction_cookie3)
        .await
        .expect("Choice 3 should be executed as it won the poll");
    let transaction4_err = governance_test
        .execute_proposal_transaction(&proposal_cookie, &proposal_transaction_cookie4)
        .await
        .expect_err("Choice 4 should be executed as the winner has been executed already");

    // Assert
    proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    assert_eq!(ProposalState::Completed, proposal_account.state);

    assert_eq!(
        transaction1_err,
        GovernanceError::CannotExecuteDefeatedOption.into()
    );
    assert_eq!(
        transaction2_err,
        GovernanceError::CannotExecuteDefeatedOption.into()
    );
    assert_eq!(
        transaction4_err,
        GovernanceError::InvalidStateCannotExecuteTransaction.into()
    );
}

#[tokio::test]
async fn test_vote_multi_weighted_choice_proposal_with_multi_success() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;

    // 100 tokens each, sum 300 tokens
    let token_owner_record_cookie1 = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();
    let token_owner_record_cookie2 = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    // 60 tokes approval quorum as 30% of 200 is 60
    let mut governance_config = governance_test.get_default_governance_config();
    governance_config.community_vote_threshold = VoteThreshold::YesVotePercentage(30);

    let mut governance_cookie = governance_test
        .with_governance_using_config(
            &realm_cookie,
            &token_owner_record_cookie1,
            &governance_config,
        )
        .await
        .unwrap();

    let governed_mint_cookie = governance_test.with_governed_mint(&governance_cookie).await;

    let mut proposal_cookie = governance_test
        .with_multi_option_proposal(
            &token_owner_record_cookie1,
            &mut governance_cookie,
            vec![
                "option 1".to_string(),
                "option 2".to_string(),
                "option 3".to_string(),
            ],
            true,
            VoteType::MultiChoice {
                choice_type: MultiChoiceType::Weighted,
                min_voter_options: 1,
                max_winning_options: 3,
                max_voter_options: 3,
            },
        )
        .await
        .unwrap();

    let proposal_transaction_cookie1 = governance_test
        .with_mint_tokens_transaction(
            &governed_mint_cookie,
            &mut proposal_cookie,
            &token_owner_record_cookie1,
            0,
            Some(0),
        )
        .await
        .unwrap();
    let proposal_transaction_cookie2 = governance_test
        .with_mint_tokens_transaction(
            &governed_mint_cookie,
            &mut proposal_cookie,
            &token_owner_record_cookie1,
            1,
            Some(0),
        )
        .await
        .unwrap();
    let proposal_transaction_cookie3 = governance_test
        .with_mint_tokens_transaction(
            &governed_mint_cookie,
            &mut proposal_cookie,
            &token_owner_record_cookie1,
            2,
            Some(0),
        )
        .await
        .unwrap();

    governance_test
        .sign_off_proposal_by_owner(&proposal_cookie, &token_owner_record_cookie1)
        .await
        .unwrap();

    // vote1 + vote2:
    //   choice 1: 28 -> Defeated (below 60)
    //   choice 2: 105 -> Success
    //   choice 3: 61 -> Success

    let vote1 = Vote::Approve(vec![
        VoteChoice {
            rank: 0,
            weight_percentage: 14,
        },
        VoteChoice {
            rank: 0,
            weight_percentage: 55,
        },
        VoteChoice {
            rank: 0,
            weight_percentage: 31,
        },
    ]);
    governance_test
        .with_cast_vote(&proposal_cookie, &token_owner_record_cookie1, vote1)
        .await
        .expect("Voting the vote 1 of owner 1 should succeed");

    let vote2 = Vote::Approve(vec![
        VoteChoice {
            rank: 0,
            weight_percentage: 20,
        },
        VoteChoice {
            rank: 0,
            weight_percentage: 50,
        },
        VoteChoice {
            rank: 0,
            weight_percentage: 30,
        },
    ]);
    governance_test
        .with_cast_vote(&proposal_cookie, &token_owner_record_cookie2, vote2)
        .await
        .expect("Voting the vote 1 of owner 1 should succeed");

    // Advance timestamp past voting_base_time
    let clock = governance_test.bench.get_clock().await;
    governance_test
        .advance_clock_past_timestamp(
            governance_cookie.account.config.voting_base_time as i64 + clock.unix_timestamp,
        )
        .await;
    governance_test
        .finalize_vote(&realm_cookie, &proposal_cookie, None)
        .await
        .unwrap();
    // Advance timestamp past hold_up_time
    governance_test
        .advance_clock_by_min_timespan(
            governance_cookie.account.config.transactions_hold_up_time as u64,
        )
        .await;

    let mut proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    assert_eq!(ProposalState::Succeeded, proposal_account.state);

    // Act
    let transaction1_err = governance_test
        .execute_proposal_transaction(&proposal_cookie, &proposal_transaction_cookie1)
        .await
        .expect_err("Choice 1 should fail to execute, it hasn't got enough votes");
    governance_test
        .execute_proposal_transaction(&proposal_cookie, &proposal_transaction_cookie2)
        .await
        .expect("Choice 2 should be executed as it passed the poll");
    governance_test
        .execute_proposal_transaction(&proposal_cookie, &proposal_transaction_cookie3)
        .await
        .expect("Choice 3 should be executed as it passed the poll");

    // Assert
    proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    assert_eq!(ProposalState::Completed, proposal_account.state);

    assert_eq!(
        transaction1_err,
        GovernanceError::CannotExecuteDefeatedOption.into()
    );
}

#[tokio::test]
async fn test_vote_multi_weighted_choice_proposal_executable_with_full_deny() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;

    // 100 tokens
    let token_owner_record_cookie1 = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    // 100 tokens
    let token_owner_record_cookie2 = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let mut governance_config = governance_test.get_default_governance_config();
    governance_config.community_vote_threshold = VoteThreshold::YesVotePercentage(3);

    let mut governance_cookie = governance_test
        .with_governance_using_config(
            &realm_cookie,
            &token_owner_record_cookie1,
            &governance_config,
        )
        .await
        .unwrap();

    let governed_mint_cookie = governance_test.with_governed_mint(&governance_cookie).await;

    let mut proposal_cookie = governance_test
        .with_multi_option_proposal(
            &token_owner_record_cookie1,
            &mut governance_cookie,
            vec!["option 1".to_string(), "option 2".to_string()],
            true,
            VoteType::MultiChoice {
                choice_type: MultiChoiceType::Weighted,
                min_voter_options: 1,
                max_winning_options: 2,
                max_voter_options: 2,
            },
        )
        .await
        .unwrap();

    let proposal_transaction_cookie1 = governance_test
        .with_mint_tokens_transaction(
            &governed_mint_cookie,
            &mut proposal_cookie,
            &token_owner_record_cookie1,
            0,
            Some(0),
        )
        .await
        .unwrap();

    let proposal_transaction_cookie2 = governance_test
        .with_mint_tokens_transaction(
            &governed_mint_cookie,
            &mut proposal_cookie,
            &token_owner_record_cookie1,
            1,
            Some(0),
        )
        .await
        .unwrap();

    governance_test
        .sign_off_proposal_by_owner(&proposal_cookie, &token_owner_record_cookie1)
        .await
        .unwrap();

    governance_test
        .with_cast_vote(&proposal_cookie, &token_owner_record_cookie1, Vote::Deny)
        .await
        .expect("Casting deny vote for owner 1 should succeed");
    governance_test
        .with_cast_vote(&proposal_cookie, &token_owner_record_cookie2, Vote::Deny)
        .await
        .expect("Casting deny vote for owner 1 should succeed");

    // Advance timestamp past voting_base_time
    let clock = governance_test.bench.get_clock().await;
    governance_test
        .advance_clock_past_timestamp(
            governance_cookie.account.config.voting_base_time as i64 + clock.unix_timestamp,
        )
        .await;

    governance_test
        .finalize_vote(&realm_cookie, &proposal_cookie, None)
        .await
        .unwrap();

    // Advance timestamp past hold_up_time
    governance_test
        .advance_clock_by_min_timespan(
            governance_cookie.account.config.transactions_hold_up_time as u64,
        )
        .await;

    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    assert_eq!(ProposalState::Defeated, proposal_account.state);

    // Act
    let transaction1_err = governance_test
        .execute_proposal_transaction(&proposal_cookie, &proposal_transaction_cookie1)
        .await
        .expect_err("The proposal was denied, error on choice 1 execution expected");
    let transaction2_err = governance_test
        .execute_proposal_transaction(&proposal_cookie, &proposal_transaction_cookie2)
        .await
        .expect_err("The proposal was denied, error on choice 2 execution expected");

    // Assert
    assert_eq!(
        transaction1_err,
        GovernanceError::InvalidStateCannotExecuteTransaction.into()
    );
    assert_eq!(
        transaction2_err,
        GovernanceError::InvalidStateCannotExecuteTransaction.into()
    );
}
