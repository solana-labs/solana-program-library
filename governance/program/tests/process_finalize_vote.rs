#![cfg(feature = "test-sbf")]

mod program_test;

use solana_program_test::tokio;

use program_test::*;
use spl_governance::{
    error::GovernanceError,
    state::{
        enums::{ProposalState, VoteThreshold},
        proposal::{OptionVoteResult, VoteType},
        vote_record::{Vote, VoteChoice},
    },
};

#[tokio::test]
async fn test_finalize_vote_to_succeeded() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    let mut governance_config = governance_test.get_default_governance_config();

    governance_config.community_vote_threshold = VoteThreshold::YesVotePercentage(40);

    let token_owner_record_cookie = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let mut governance_cookie = governance_test
        .with_governance_using_config(
            &realm_cookie,
            &governed_account_cookie,
            &token_owner_record_cookie,
            &governance_config,
        )
        .await
        .unwrap();

    // Total 210 tokens
    governance_test
        .mint_community_tokens(&realm_cookie, 110)
        .await;

    let proposal_cookie = governance_test
        .with_signed_off_proposal(&token_owner_record_cookie, &mut governance_cookie)
        .await
        .unwrap();

    governance_test
        .with_cast_yes_no_vote(&proposal_cookie, &token_owner_record_cookie, YesNoVote::Yes)
        .await
        .unwrap();

    // Ensure not tipped
    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    assert_eq!(ProposalState::Voting, proposal_account.state);

    // Advance timestamp past max_voting_time
    governance_test
        .advance_clock_past_timestamp(
            governance_cookie.account.config.voting_base_time as i64
                + proposal_account.voting_at.unwrap(),
        )
        .await;

    // Act

    governance_test
        .finalize_vote(&realm_cookie, &proposal_cookie, None)
        .await
        .unwrap();

    // Assert

    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    assert_eq!(proposal_account.state, ProposalState::Succeeded);
    assert_eq!(
        Some(proposal_account.voting_max_time_end(&governance_cookie.account.config)),
        proposal_account.voting_completed_at
    );

    assert_eq!(Some(210), proposal_account.max_vote_weight);

    assert_eq!(
        Some(governance_cookie.account.config.community_vote_threshold),
        proposal_account.vote_threshold
    );

    let proposal_owner_record = governance_test
        .get_token_owner_record_account(&proposal_cookie.account.token_owner_record)
        .await;

    assert_eq!(0, proposal_owner_record.outstanding_proposal_count);

    let governance_account = governance_test
        .get_governance_account(&governance_cookie.address)
        .await;

    assert_eq!(0, governance_account.active_proposal_count);
}

#[tokio::test]
async fn test_finalize_vote_to_defeated() {
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

    governance_test
        .with_cast_yes_no_vote(&proposal_cookie, &token_owner_record_cookie, YesNoVote::No)
        .await
        .unwrap();

    // Ensure not tipped
    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    assert_eq!(ProposalState::Voting, proposal_account.state);

    // Advance clock past max_voting_time
    governance_test
        .advance_clock_past_timestamp(
            governance_cookie.account.config.voting_base_time as i64
                + proposal_account.voting_at.unwrap(),
        )
        .await;

    // Act

    governance_test
        .finalize_vote(&realm_cookie, &proposal_cookie, None)
        .await
        .unwrap();

    // Assert

    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    assert_eq!(ProposalState::Defeated, proposal_account.state);
}

#[tokio::test]
async fn test_finalize_vote_with_invalid_mint_error() {
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

    let mut proposal_cookie = governance_test
        .with_signed_off_proposal(&token_owner_record_cookie, &mut governance_cookie)
        .await
        .unwrap();

    governance_test
        .with_cast_yes_no_vote(&proposal_cookie, &token_owner_record_cookie, YesNoVote::No)
        .await
        .unwrap();

    // Ensure not tipped
    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    assert_eq!(ProposalState::Voting, proposal_account.state);

    proposal_cookie.account.governing_token_mint =
        realm_cookie.account.config.council_mint.unwrap();

    // Act

    let err = governance_test
        .finalize_vote(&realm_cookie, &proposal_cookie, None)
        .await
        .err()
        .unwrap();

    // Assert

    assert_eq!(err, GovernanceError::InvalidGoverningMintForProposal.into());
}

#[tokio::test]
async fn test_finalize_vote_with_invalid_governance_error() {
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

    let mut proposal_cookie = governance_test
        .with_signed_off_proposal(&token_owner_record_cookie, &mut governance_cookie)
        .await
        .unwrap();

    governance_test
        .with_cast_yes_no_vote(&proposal_cookie, &token_owner_record_cookie, YesNoVote::No)
        .await
        .unwrap();

    // Ensure not tipped
    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    assert_eq!(ProposalState::Voting, proposal_account.state);

    // Setup Governance for a different account
    let governed_account_cookie2 = governance_test.with_governed_account().await;

    let governance_cookie2 = governance_test
        .with_governance(
            &realm_cookie,
            &governed_account_cookie2,
            &token_owner_record_cookie,
        )
        .await
        .unwrap();

    proposal_cookie.account.governance = governance_cookie2.address;

    // Act

    let err = governance_test
        .finalize_vote(&realm_cookie, &proposal_cookie, None)
        .await
        .err()
        .unwrap();

    // Assert

    assert_eq!(err, GovernanceError::InvalidGovernanceForProposal.into());
}

#[tokio::test]
async fn test_finalize_council_vote() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    let mut governance_config = governance_test.get_default_governance_config();
    governance_config.council_vote_threshold = VoteThreshold::YesVotePercentage(40);
    governance_config.community_vote_threshold = VoteThreshold::Disabled;

    // Deposit 100 council tokens
    let token_owner_record_cookie = governance_test
        .with_council_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let mut governance_cookie = governance_test
        .with_governance_using_config(
            &realm_cookie,
            &governed_account_cookie,
            &token_owner_record_cookie,
            &governance_config,
        )
        .await
        .unwrap();

    // Total 210 council tokens in circulation
    governance_test
        .mint_council_tokens(&realm_cookie, 110)
        .await;

    let proposal_cookie = governance_test
        .with_signed_off_proposal(&token_owner_record_cookie, &mut governance_cookie)
        .await
        .unwrap();

    // Cast vote with 47% weight, above 40% quorum but below 50%+1 to tip automatically
    governance_test
        .with_cast_yes_no_vote(&proposal_cookie, &token_owner_record_cookie, YesNoVote::Yes)
        .await
        .unwrap();

    // Ensure not tipped
    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    assert_eq!(ProposalState::Voting, proposal_account.state);

    // Advance timestamp past max_voting_time
    governance_test
        .advance_clock_past_timestamp(
            governance_cookie.account.config.voting_base_time as i64
                + proposal_account.voting_at.unwrap(),
        )
        .await;

    // Act

    governance_test
        .finalize_vote(&realm_cookie, &proposal_cookie, None)
        .await
        .unwrap();

    // Assert

    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    assert_eq!(proposal_account.state, ProposalState::Succeeded);
    assert_eq!(
        Some(proposal_account.voting_max_time_end(&governance_cookie.account.config)),
        proposal_account.voting_completed_at
    );

    assert_eq!(Some(210), proposal_account.max_vote_weight);

    assert_eq!(
        Some(governance_cookie.account.config.council_vote_threshold),
        proposal_account.vote_threshold
    );
}

#[tokio::test]
async fn test_finalize_vote_with_cannot_finalize_during_voting_time_error() {
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

    // Total 210 tokens
    governance_test
        .mint_community_tokens(&realm_cookie, 110)
        .await;

    let proposal_cookie = governance_test
        .with_signed_off_proposal(&token_owner_record_cookie, &mut governance_cookie)
        .await
        .unwrap();

    governance_test
        .with_cast_yes_no_vote(&proposal_cookie, &token_owner_record_cookie, YesNoVote::Yes)
        .await
        .unwrap();

    governance_test.advance_clock().await;

    // Act

    let err = governance_test
        .finalize_vote(&realm_cookie, &proposal_cookie, None)
        .await
        .err()
        .unwrap();

    // Assert

    assert_eq!(err, GovernanceError::CannotFinalizeVotingInProgress.into());
}

#[tokio::test]
async fn test_finalize_vote_with_cannot_finalize_during_cool_off_time_error() {
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

    // Total 210 tokens
    governance_test
        .mint_community_tokens(&realm_cookie, 110)
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

    let err = governance_test
        .finalize_vote(&realm_cookie, &proposal_cookie, None)
        .await
        .err()
        .unwrap();

    // Assert

    assert_eq!(err, GovernanceError::CannotFinalizeVotingInProgress.into());
}

#[tokio::test]
async fn test_finalize_vote_attendance_quorum() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    let mut governance_config = governance_test.get_default_governance_config();

    // Attendance quorum, all mint owners mustvote (100% of them), only 30% needed for an option being successful
    // Total 300 tokens; one token owner owns 100 that will be voted with
    governance_config.community_vote_threshold = VoteThreshold::AttendanceQuorum {
        threshold: 10000,
        pass_level: 30,
    };

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

    let mut governance_cookie = governance_test
        .with_governance_using_config(
            &realm_cookie,
            &governed_account_cookie,
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
            true, // no transactions but not a survey
            VoteType::MultiChoice {
                max_winning_options: 3,
                max_voter_options: 3,
            },
        )
        .await
        .unwrap();

    governance_test
        .sign_off_proposal_by_owner(&proposal_cookie, &token_owner_record_cookie1)
        .await
        .unwrap();

    let vote1 = Vote::Approve(vec![
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
    let vote2 = Vote::Approve(vec![
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
    let vote3 = Vote::Approve(vec![
        VoteChoice {
            rank: 0,
            weight_percentage: 0,
        },
        VoteChoice {
            rank: 0,
            weight_percentage: 0,
        },
        VoteChoice {
            rank: 0,
            weight_percentage: 100,
        },
    ]);

    governance_test
        .with_cast_vote(&proposal_cookie, &token_owner_record_cookie1, vote1)
        .await
        .unwrap();
    governance_test
        .with_cast_vote(&proposal_cookie, &token_owner_record_cookie2, vote2)
        .await
        .unwrap();
    governance_test
        .with_cast_vote(&proposal_cookie, &token_owner_record_cookie3, vote3)
        .await
        .unwrap();

    // Ensure not tipped
    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    assert_eq!(ProposalState::Voting, proposal_account.state);

    // advance time
    let clock = governance_test.bench.get_clock().await;
    governance_test
        .advance_clock_past_timestamp(
            clock.unix_timestamp + governance_cookie.account.config.voting_base_time as i64,
        )
        .await;

    // Act
    governance_test
        .finalize_vote(&realm_cookie, &proposal_cookie, None)
        .await
        .unwrap();

    // Assert
    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    // end state of non-survey proposal is succeeded
    assert_eq!(proposal_account.state, ProposalState::Succeeded);
    assert_eq!(Some(300), proposal_account.max_vote_weight);
    for option in proposal_account.options {
        assert_eq!(OptionVoteResult::Succeeded, option.vote_result);
    }
}

#[tokio::test]
async fn test_finalize_vote_attendance_quorum_as_survey() {
    // the survey "type" of the proposal is one that contains no transactions
    // and permits no deny votes
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    let mut governance_config = governance_test.get_default_governance_config();

    // Attendance quorum, 50% of owners has to vote, 50% needed for an option to be successful
    // Total 300 tokens; one token owner owns 100 that will be voted with
    governance_config.community_vote_threshold = VoteThreshold::AttendanceQuorum {
        threshold: 5000,
        pass_level: 100, // survey does not consider pass level
    };

    let token_owner_record_cookie1 = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();
    let token_owner_record_cookie2 = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();
    governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let mut governance_cookie = governance_test
        .with_governance_using_config(
            &realm_cookie,
            &governed_account_cookie,
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
            false, // a survey
            VoteType::MultiChoice {
                max_winning_options: 3,
                max_voter_options: 3,
            },
        )
        .await
        .unwrap();

    governance_test
        .sign_off_proposal_by_owner(&proposal_cookie, &token_owner_record_cookie1)
        .await
        .unwrap();

    let vote1 = Vote::Approve(vec![
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
    let vote2 = Vote::Approve(vec![
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
    governance_test
        .with_cast_vote(&proposal_cookie, &token_owner_record_cookie2, vote2)
        .await
        .unwrap();

    // Ensure not tipped
    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;
    assert_eq!(ProposalState::Voting, proposal_account.state);

    // advance time
    let clock = governance_test.bench.get_clock().await;
    governance_test
        .advance_clock_past_timestamp(
            clock.unix_timestamp + governance_cookie.account.config.voting_base_time as i64,
        )
        .await;

    // Act
    governance_test
        .finalize_vote(&realm_cookie, &proposal_cookie, None)
        .await
        .unwrap();

    // Assert
    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    // the transition move from Succeeded to Completed happens as deny vote is not permitted
    assert_eq!(proposal_account.state, ProposalState::Completed);
    assert_eq!(Some(300), proposal_account.max_vote_weight);
    assert_eq!(
        OptionVoteResult::None,
        proposal_account.options[0].vote_result
    );
    assert_eq!(
        OptionVoteResult::None,
        proposal_account.options[1].vote_result
    );
    assert_eq!(
        OptionVoteResult::None,
        proposal_account.options[2].vote_result
    );
}

#[tokio::test]
async fn test_finalize_vote_attendance_quorum_fail_threshold() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    let mut governance_config = governance_test.get_default_governance_config();

    // Attendance quorum, all must vote
    governance_config.community_vote_threshold = VoteThreshold::AttendanceQuorum {
        threshold: 10000,
        pass_level: 0,
    };

    // 300 tokens for governance mint but no votes cast
    let token_owner_record_cookie1 = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();
    let token_owner_record_cookie2 = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();
    governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let mut governance_cookie = governance_test
        .with_governance_using_config(
            &realm_cookie,
            &governed_account_cookie,
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
            true, // no transactions but not a survey
            VoteType::MultiChoice {
                max_winning_options: 3,
                max_voter_options: 3,
            },
        )
        .await
        .unwrap();

    governance_test
        .sign_off_proposal_by_owner(&proposal_cookie, &token_owner_record_cookie1)
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
        VoteChoice {
            rank: 0,
            weight_percentage: 0,
        },
    ]);
    governance_test
        .with_cast_vote(&proposal_cookie, &token_owner_record_cookie2, vote)
        .await
        .unwrap();

    // advance time
    let clock = governance_test.bench.get_clock().await;
    governance_test
        .advance_clock_past_timestamp(
            clock.unix_timestamp + governance_cookie.account.config.voting_base_time as i64,
        )
        .await;

    // Act
    governance_test
        .finalize_vote(&realm_cookie, &proposal_cookie, None)
        .await
        .unwrap();

    // Assert
    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    // end state of non-survey proposal is succeeded
    assert_eq!(proposal_account.state, ProposalState::Defeated);
    assert_eq!(Some(300), proposal_account.max_vote_weight);
    for option in proposal_account.options {
        assert_eq!(OptionVoteResult::Defeated, option.vote_result);
    }
}

#[tokio::test]
async fn test_finalize_vote_attendance_quorum_survey_fail_threshold() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    let mut governance_config = governance_test.get_default_governance_config();

    // Attendance quorum, somebody must vote
    governance_config.community_vote_threshold = VoteThreshold::AttendanceQuorum {
        threshold: 1,
        pass_level: 0,
    };

    // 300 tokens for governance mint but no votes cast
    let token_owner_record_cookie1 = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let mut governance_cookie = governance_test
        .with_governance_using_config(
            &realm_cookie,
            &governed_account_cookie,
            &token_owner_record_cookie1,
            &governance_config,
        )
        .await
        .unwrap();

    let proposal_cookie = governance_test
        .with_multi_option_proposal(
            &token_owner_record_cookie1,
            &mut governance_cookie,
            vec!["option 1".to_string(), "option 2".to_string()],
            false, // a survey (no transactions, no deny votes)
            VoteType::MultiChoice {
                max_winning_options: 2,
                max_voter_options: 2,
            },
        )
        .await
        .unwrap();

    governance_test
        .sign_off_proposal_by_owner(&proposal_cookie, &token_owner_record_cookie1)
        .await
        .unwrap();

    // advance time
    let clock = governance_test.bench.get_clock().await;
    governance_test
        .advance_clock_past_timestamp(
            clock.unix_timestamp + governance_cookie.account.config.voting_base_time as i64,
        )
        .await;

    // Act
    governance_test
        .finalize_vote(&realm_cookie, &proposal_cookie, None)
        .await
        .unwrap();

    // Assert
    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    // end state of non-survey proposal is succeeded
    assert_eq!(proposal_account.state, ProposalState::Defeated);
    assert_eq!(Some(100), proposal_account.max_vote_weight);
    for option in proposal_account.options {
        assert_eq!(OptionVoteResult::None, option.vote_result);
    }
}

#[tokio::test]
async fn test_finalize_vote_attendance_quorum_pass_level_0() {
    // Arrange
    let mut governance_test = GovernanceProgramTest::start_new().await;

    let realm_cookie = governance_test.with_realm().await;
    let governed_account_cookie = governance_test.with_governed_account().await;

    let mut governance_config = governance_test.get_default_governance_config();

    // Attendance quorum, 50% of the votes must vote
    governance_config.community_vote_threshold = VoteThreshold::AttendanceQuorum {
        threshold: 50,
        pass_level: 0, // pass level 0 still means 0% of the votes failed
    };

    // 300 tokens for governance mint but no votes cast
    let token_owner_record_cookie1 = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();
    let token_owner_record_cookie2 = governance_test
        .with_community_token_deposit(&realm_cookie)
        .await
        .unwrap();

    let mut governance_cookie = governance_test
        .with_governance_using_config(
            &realm_cookie,
            &governed_account_cookie,
            &token_owner_record_cookie1,
            &governance_config,
        )
        .await
        .unwrap();

    let proposal_cookie = governance_test
        .with_multi_option_proposal(
            &token_owner_record_cookie1,
            &mut governance_cookie,
            vec!["option 1".to_string(), "option 2".to_string()],
            true, // no transactions but not a survey
            VoteType::MultiChoice {
                max_winning_options: 2,
                max_voter_options: 2,
            },
        )
        .await
        .unwrap();

    governance_test
        .sign_off_proposal_by_owner(&proposal_cookie, &token_owner_record_cookie1)
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
    governance_test
        .with_cast_vote(&proposal_cookie, &token_owner_record_cookie2, vote)
        .await
        .unwrap();

    // advance time
    let clock = governance_test.bench.get_clock().await;
    governance_test
        .advance_clock_past_timestamp(
            clock.unix_timestamp + governance_cookie.account.config.voting_base_time as i64,
        )
        .await;

    // Act
    governance_test
        .finalize_vote(&realm_cookie, &proposal_cookie, None)
        .await
        .unwrap();

    // Assert
    let proposal_account = governance_test
        .get_proposal_account(&proposal_cookie.address)
        .await;

    // end state of non-survey proposal is succeeded
    assert_eq!(proposal_account.state, ProposalState::Succeeded);
    assert_eq!(Some(200), proposal_account.max_vote_weight);
    assert_eq!(
        OptionVoteResult::Succeeded,
        proposal_account.options[0].vote_result
    );
    assert_eq!(
        OptionVoteResult::Defeated,
        proposal_account.options[1].vote_result
    );
}
