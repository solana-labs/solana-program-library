//! Proposal  Account

use solana_program::{
    account_info::AccountInfo, epoch_schedule::Slot, program_error::ProgramError,
    program_pack::IsInitialized, pubkey::Pubkey,
};

use crate::{
    error::GovernanceError,
    id,
    tools::account::{get_account_data, AccountMaxSize},
    PROGRAM_AUTHORITY_SEED,
};

use crate::state::enums::{GovernanceAccountType, ProposalState};
use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};

use crate::state::governance::GovernanceConfig;

/// Governance Proposal
#[repr(C)]
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub struct Proposal {
    /// Governance account type
    pub account_type: GovernanceAccountType,

    /// Governance account the Proposal belongs to
    pub governance: Pubkey,

    /// Indicates which Governing Token is used to vote on the Proposal
    /// Whether the general Community token owners or the Council tokens owners vote on this Proposal
    pub governing_token_mint: Pubkey,

    /// Current proposal state
    pub state: ProposalState,

    /// The TokenOwnerRecord representing the user who created and owns this Proposal
    pub token_owner_record: Pubkey,

    /// The number of signatories assigned to the Proposal
    pub signatories_count: u8,

    /// The number of signatories who already signed
    pub signatories_signed_off_count: u8,

    /// Link to proposal's description
    pub description_link: String,

    /// Proposal name
    pub name: String,

    /// The number of Yes votes
    pub yes_votes_count: u64,

    /// The number of No votes
    pub no_votes_count: u64,

    /// When the Proposal was created and entered Draft state
    pub draft_at: Slot,

    /// When Signatories started signing off the Proposal
    pub signing_off_at: Option<Slot>,

    /// When the Proposal began voting
    pub voting_at: Option<Slot>,

    /// When the Proposal ended voting and entered either Succeeded or Defeated
    pub voting_completed_at: Option<Slot>,

    /// When the Proposal entered Executing state
    pub executing_at: Option<Slot>,

    /// When the Proposal entered final state Completed or Cancelled and was closed
    pub closed_at: Option<Slot>,

    /// The number of the instructions already executed
    pub number_of_executed_instructions: u8,

    /// The number of instructions included in the proposal
    pub number_of_instructions: u8,
}

impl AccountMaxSize for Proposal {
    fn get_max_size(&self) -> Option<usize> {
        Some(self.name.len() + self.description_link.len() + 179)
    }
}

impl IsInitialized for Proposal {
    fn is_initialized(&self) -> bool {
        self.account_type == GovernanceAccountType::Proposal
    }
}

impl Proposal {
    /// Checks if Signatories can be edited (added or removed) for the Proposal in the given state
    pub fn assert_can_edit_signatories(&self) -> Result<(), ProgramError> {
        self.assert_is_draft_state()
            .map_err(|_| GovernanceError::InvalidStateCannotEditSignatories.into())
    }

    /// Checks if Proposal can be singed off
    pub fn assert_can_sign_off(&self) -> Result<(), ProgramError> {
        match self.state {
            ProposalState::Draft | ProposalState::SigningOff => Ok(()),
            ProposalState::Executing
            | ProposalState::Completed
            | ProposalState::Cancelled
            | ProposalState::Voting
            | ProposalState::Succeeded
            | ProposalState::Defeated => Err(GovernanceError::InvalidStateCannotSignOff.into()),
        }
    }

    /// Checks the Proposal is in Voting state
    fn assert_is_voting_state(&self) -> Result<(), ProgramError> {
        if self.state != ProposalState::Voting {
            return Err(GovernanceError::InvalidProposalState.into());
        }

        Ok(())
    }

    /// Checks the Proposal is in Draft state
    fn assert_is_draft_state(&self) -> Result<(), ProgramError> {
        if self.state != ProposalState::Draft {
            return Err(GovernanceError::InvalidProposalState.into());
        }

        Ok(())
    }

    /// Checks if Proposal can be voted on
    pub fn assert_can_cast_vote(
        &self,
        config: &GovernanceConfig,
        current_slot: Slot,
    ) -> Result<(), ProgramError> {
        self.assert_is_voting_state()
            .map_err(|_| GovernanceError::InvalidStateCannotVote)?;

        // Check if we are still within the configured max_voting_time period
        if self
            .voting_at
            .unwrap()
            .checked_add(config.max_voting_time)
            .unwrap()
            < current_slot
        {
            return Err(GovernanceError::ProposalVotingTimeExpired.into());
        }

        Ok(())
    }

    /// Checks if Proposal can be finalized
    pub fn assert_can_finalize_vote(
        &self,
        config: &GovernanceConfig,
        current_slot: Slot,
    ) -> Result<(), ProgramError> {
        self.assert_is_voting_state()
            .map_err(|_| GovernanceError::InvalidStateCannotFinalize)?;

        // Check if we passed the configured max_voting_time period yet
        if self
            .voting_at
            .unwrap()
            .checked_add(config.max_voting_time)
            .unwrap()
            >= current_slot
        {
            return Err(GovernanceError::CannotFinalizeVotingInProgress.into());
        }

        Ok(())
    }

    /// Finalizes vote by moving it to final state Succeeded or Defeated if max_voting_time has passed
    /// If Proposal is still within max_voting_time period then error is returned
    pub fn finalize_vote(
        &mut self,
        governing_token_supply: u64,
        config: &GovernanceConfig,
        current_slot: Slot,
    ) -> Result<(), ProgramError> {
        self.assert_can_finalize_vote(config, current_slot)?;

        let current_yes_vote_threshold =
            get_vote_threshold(self.yes_votes_count, governing_token_supply);

        let final_state =
            if current_yes_vote_threshold > config.yes_vote_threshold_percentage as u64 {
                ProposalState::Succeeded
            } else {
                ProposalState::Defeated
            };

        self.state = final_state;
        self.voting_completed_at = Some(current_slot);

        Ok(())
    }

    /// Checks if vote can be tipped and automatically transitioned to Succeeded or Defeated state
    /// If the conditions are met the state is updated accordingly
    pub fn try_tip_vote(
        &mut self,
        governing_token_supply: u64,
        config: &GovernanceConfig,
        current_slot: Slot,
    ) {
        if let Some(tipped_state) = self.try_get_tipped_vote_state(governing_token_supply, config) {
            self.state = tipped_state;
            self.voting_completed_at = Some(current_slot);
        }
    }

    /// Checks if vote can be tipped and automatically transitioned to Succeeded or Defeated state
    /// If yes then Some(ProposalState) is returned and None otherwise
    pub fn try_get_tipped_vote_state(
        &self,
        governing_token_supply: u64,
        config: &GovernanceConfig,
    ) -> Option<ProposalState> {
        if self.yes_votes_count == governing_token_supply {
            return Some(ProposalState::Succeeded);
        }
        if self.no_votes_count == governing_token_supply {
            return Some(ProposalState::Defeated);
        }

        let current_yes_vote_threshold =
            get_vote_threshold(self.yes_votes_count, governing_token_supply);

        // We can only tip the vote automatically to Succeeded if more than 50% votes have been cast as Yeah
        // and the Nay sayers can't change the outcome any longer
        if current_yes_vote_threshold >= 50
            && current_yes_vote_threshold > config.yes_vote_threshold_percentage as u64
        {
            return Some(ProposalState::Succeeded);
        } else {
            let current_no_vote_threshold =
                get_vote_threshold(self.no_votes_count, governing_token_supply);

            // We can  tip the vote automatically to Defeated if more than 50% votes have been cast as Nay
            // or the Yeah sayers can't outvote Nay any longer
            // Note: Even splits  resolve to Defeated
            if current_no_vote_threshold >= 50
                || current_no_vote_threshold >= 100 - config.yes_vote_threshold_percentage as u64
            {
                return Some(ProposalState::Defeated);
            }
        }

        None
    }

    /// Checks if Proposal can be canceled in the given state
    pub fn assert_can_cancel(&self) -> Result<(), ProgramError> {
        match self.state {
            ProposalState::Draft | ProposalState::SigningOff | ProposalState::Voting => Ok(()),
            ProposalState::Executing
            | ProposalState::Completed
            | ProposalState::Cancelled
            | ProposalState::Succeeded
            | ProposalState::Defeated => {
                Err(GovernanceError::InvalidStateCannotCancelProposal.into())
            }
        }
    }
}

/// Returns current vote threshold in relation to the total governing token supply
fn get_vote_threshold(vote_count: u64, governing_token_supply: u64) -> u64 {
    (vote_count as u128)
        .checked_mul(100)
        .unwrap()
        .checked_div(governing_token_supply as u128)
        .unwrap() as u64
}

/// Deserializes Proposal account and checks owner program
pub fn get_proposal_data(proposal_info: &AccountInfo) -> Result<Proposal, ProgramError> {
    get_account_data::<Proposal>(proposal_info, &id())
}

/// Deserializes Proposal and validates it belongs to the given Governance and Governing Mint
pub fn get_proposal_data_for_governance_and_governing_mint(
    proposal_info: &AccountInfo,
    governance: &Pubkey,
    governing_token_mint: &Pubkey,
) -> Result<Proposal, ProgramError> {
    let proposal_data = get_proposal_data(proposal_info)?;

    if proposal_data.governance != *governance {
        return Err(GovernanceError::InvalidGovernanceForProposal.into());
    }

    if proposal_data.governing_token_mint != *governing_token_mint {
        return Err(GovernanceError::InvalidGoverningMintForProposal.into());
    }

    Ok(proposal_data)
}

/// Returns Proposal PDA seeds
pub fn get_proposal_address_seeds<'a>(
    governance: &'a Pubkey,
    governing_token_mint: &'a Pubkey,
    proposal_index_le_bytes: &'a [u8],
) -> [&'a [u8]; 4] {
    [
        PROGRAM_AUTHORITY_SEED,
        governance.as_ref(),
        governing_token_mint.as_ref(),
        &proposal_index_le_bytes,
    ]
}

/// Returns Proposal PDA address
pub fn get_proposal_address<'a>(
    governance: &'a Pubkey,
    governing_token_mint: &'a Pubkey,
    proposal_index_le_bytes: &'a [u8],
) -> Pubkey {
    Pubkey::find_program_address(
        &get_proposal_address_seeds(governance, governing_token_mint, &proposal_index_le_bytes),
        &id(),
    )
    .0
}

#[cfg(test)]
mod test {

    use {super::*, proptest::prelude::*};

    fn create_test_proposal() -> Proposal {
        Proposal {
            account_type: GovernanceAccountType::TokenOwnerRecord,
            governance: Pubkey::new_unique(),
            governing_token_mint: Pubkey::new_unique(),
            state: ProposalState::Draft,
            token_owner_record: Pubkey::new_unique(),
            signatories_count: 10,
            signatories_signed_off_count: 5,
            description_link: "This is my description".to_string(),
            name: "This is my name".to_string(),
            draft_at: 10,
            signing_off_at: Some(10),
            voting_at: Some(10),
            voting_completed_at: Some(10),
            executing_at: Some(10),
            closed_at: Some(10),
            number_of_executed_instructions: 10,
            number_of_instructions: 10,
            yes_votes_count: 0,
            no_votes_count: 0,
        }
    }

    fn create_test_governance_config() -> GovernanceConfig {
        GovernanceConfig {
            realm: Pubkey::new_unique(),
            governed_account: Pubkey::new_unique(),
            yes_vote_threshold_percentage: 60,
            min_tokens_to_create_proposal: 5,
            min_instruction_hold_up_time: 10,
            max_voting_time: 5,
        }
    }

    #[test]
    fn test_max_size() {
        let proposal = create_test_proposal();
        let size = proposal.try_to_vec().unwrap().len();

        assert_eq!(proposal.get_max_size(), Some(size));
    }

    #[test]
    fn test_get_vote_threshold_within_max_bounds() {
        let result = get_vote_threshold(u64::MAX, u64::MAX);
        assert_eq!(result, 100);
    }

    prop_compose! {
        fn vote_results()(governing_token_supply in 1..=u64::MAX)(
            governing_token_supply in Just(governing_token_supply),
            vote_count in 0..=governing_token_supply,
        ) -> (u64, u64) {
            (vote_count as u64, governing_token_supply as u64)
        }
    }

    proptest! {
        #[test]
        fn test_get_vote_threshold(
            (vote_count, governing_token_supply) in vote_results(),

        ) {
            let result = get_vote_threshold(vote_count, governing_token_supply);

            assert_eq!(true, result <= 100);
        }
    }

    fn editable_signatory_states() -> impl Strategy<Value = ProposalState> {
        prop_oneof![Just(ProposalState::Draft)]
    }

    proptest! {
        #[test]
        fn test_assert_can_edit_signatories(state in editable_signatory_states()) {

            let mut proposal = create_test_proposal();
            proposal.state = state;
            proposal.assert_can_edit_signatories().unwrap();

        }

    }

    fn none_editable_signatory_states() -> impl Strategy<Value = ProposalState> {
        prop_oneof![
            Just(ProposalState::Voting),
            Just(ProposalState::Succeeded),
            Just(ProposalState::Executing),
            Just(ProposalState::Completed),
            Just(ProposalState::Cancelled),
            Just(ProposalState::Defeated),
            Just(ProposalState::SigningOff),
        ]
    }

    proptest! {
        #[test]
            fn test_assert_can_edit_signatories_with_invalid_state_error(state in none_editable_signatory_states()) {
                // Arrange
                let mut proposal = create_test_proposal();
                proposal.state = state;

                // Act
                let err = proposal.assert_can_edit_signatories().err().unwrap();

                // Assert
                assert_eq!(err, GovernanceError::InvalidStateCannotEditSignatories.into());
        }

    }

    fn sign_off_states() -> impl Strategy<Value = ProposalState> {
        prop_oneof![Just(ProposalState::SigningOff), Just(ProposalState::Draft),]
    }
    proptest! {
        #[test]
        fn test_assert_can_sign_off(state in sign_off_states()) {
            let mut proposal = create_test_proposal();
            proposal.state = state;
            proposal.assert_can_sign_off().unwrap();
        }
    }

    fn none_sign_off_states() -> impl Strategy<Value = ProposalState> {
        prop_oneof![
            Just(ProposalState::Voting),
            Just(ProposalState::Succeeded),
            Just(ProposalState::Executing),
            Just(ProposalState::Completed),
            Just(ProposalState::Cancelled),
            Just(ProposalState::Defeated),
        ]
    }

    proptest! {
        #[test]
        fn test_assert_can_sign_off_with_state_error(state in none_sign_off_states()) {
                // Arrange
                let mut proposal = create_test_proposal();
                proposal.state = state;

                // Act
                let err = proposal.assert_can_sign_off().err().unwrap();

                // Assert
                assert_eq!(err, GovernanceError::InvalidStateCannotSignOff.into());
        }
    }

    fn cancellable_states() -> impl Strategy<Value = ProposalState> {
        prop_oneof![
            Just(ProposalState::Draft),
            Just(ProposalState::SigningOff),
            Just(ProposalState::Voting),
        ]
    }

    proptest! {
        #[test]
        fn test_assert_can_cancel(state in cancellable_states()) {

            let mut proposal = create_test_proposal();
            proposal.state = state;
            proposal.assert_can_cancel().unwrap();

        }

    }

    fn none_cancellable_states() -> impl Strategy<Value = ProposalState> {
        prop_oneof![
            Just(ProposalState::Succeeded),
            Just(ProposalState::Executing),
            Just(ProposalState::Completed),
            Just(ProposalState::Cancelled),
            Just(ProposalState::Defeated),
        ]
    }

    proptest! {
        #[test]
            fn test_assert_can_cancel_with_invalid_state_error(state in none_cancellable_states()) {
                // Arrange
                let mut proposal = create_test_proposal();
                proposal.state = state;

                // Act
                let err = proposal.assert_can_cancel().err().unwrap();

                // Assert
                assert_eq!(err, GovernanceError::InvalidStateCannotCancelProposal.into());
        }

    }

    #[derive(Clone, Debug)]
    pub struct VoteTippingTestCase {
        governing_token_supply: u64,
        vote_threshold_percentage: u8,
        yes_votes_count: u64,
        no_votes_count: u64,
        expected_state: ProposalState,
    }

    fn vote_tipping_test_cases() -> impl Strategy<Value = VoteTippingTestCase> {
        prop_oneof![
            //  threshold < 50%
            Just(VoteTippingTestCase {
                governing_token_supply: 100,
                vote_threshold_percentage: 40,
                yes_votes_count: 45,
                no_votes_count: 10,
                expected_state: ProposalState::Voting
            }),
            Just(VoteTippingTestCase {
                governing_token_supply: 100,
                vote_threshold_percentage: 40,
                yes_votes_count: 50,
                no_votes_count: 10,
                expected_state: ProposalState::Succeeded
            }),
            Just(VoteTippingTestCase {
                governing_token_supply: 100,
                vote_threshold_percentage: 40,
                yes_votes_count: 50,
                no_votes_count: 50,
                expected_state: ProposalState::Succeeded
            }),
            Just(VoteTippingTestCase {
                governing_token_supply: 100,
                vote_threshold_percentage: 40,
                yes_votes_count: 45,
                no_votes_count: 51,
                expected_state: ProposalState::Defeated
            }),
            // threshold >= 50%
            Just(VoteTippingTestCase {
                governing_token_supply: 100,
                vote_threshold_percentage: 50,
                yes_votes_count: 50,
                no_votes_count: 10,
                expected_state: ProposalState::Voting
            }),
            Just(VoteTippingTestCase {
                governing_token_supply: 100,
                vote_threshold_percentage: 50,
                yes_votes_count: 50,
                no_votes_count: 50,
                expected_state: ProposalState::Defeated
            }),
            Just(VoteTippingTestCase {
                governing_token_supply: 100,
                vote_threshold_percentage: 50,
                yes_votes_count: 51,
                no_votes_count: 10,
                expected_state: ProposalState::Succeeded
            }),
            Just(VoteTippingTestCase {
                governing_token_supply: 100,
                vote_threshold_percentage: 50,
                yes_votes_count: 10,
                no_votes_count: 51,
                expected_state: ProposalState::Defeated
            }),
            Just(VoteTippingTestCase {
                governing_token_supply: 100,
                vote_threshold_percentage: 60,
                yes_votes_count: 10,
                no_votes_count: 10,
                expected_state: ProposalState::Voting
            }),
            Just(VoteTippingTestCase {
                governing_token_supply: 100,
                vote_threshold_percentage: 60,
                yes_votes_count: 60,
                no_votes_count: 10,
                expected_state: ProposalState::Voting
            }),
            Just(VoteTippingTestCase {
                governing_token_supply: 100,
                vote_threshold_percentage: 60,
                yes_votes_count: 61,
                no_votes_count: 10,
                expected_state: ProposalState::Succeeded
            }),
            Just(VoteTippingTestCase {
                governing_token_supply: 100,
                vote_threshold_percentage: 60,
                yes_votes_count: 10,
                no_votes_count: 40,
                expected_state: ProposalState::Defeated
            }),
            Just(VoteTippingTestCase {
                governing_token_supply: 100,
                vote_threshold_percentage: 60,
                yes_votes_count: 10,
                no_votes_count: 41,
                expected_state: ProposalState::Defeated
            }),
            // 100% Yes
            Just(VoteTippingTestCase {
                governing_token_supply: 100,
                vote_threshold_percentage: 100,
                yes_votes_count: 100,
                no_votes_count: 0,
                expected_state: ProposalState::Succeeded
            }),
            // 100% No
            Just(VoteTippingTestCase {
                governing_token_supply: 100,
                vote_threshold_percentage: 100,
                yes_votes_count: 0,
                no_votes_count: 100,
                expected_state: ProposalState::Defeated
            }),
        ]
    }

    proptest! {
        #[test]
        fn test_try_tip_vote(test_case in vote_tipping_test_cases()) {
            // Arrange
            let mut proposal = create_test_proposal();
            proposal.yes_votes_count = test_case.yes_votes_count;
            proposal.no_votes_count = test_case.no_votes_count;
            proposal.state = ProposalState::Voting;

            let mut governance_config = create_test_governance_config();
            governance_config.yes_vote_threshold_percentage = test_case.vote_threshold_percentage;

            let current_slot = 15_u64;

            // Act
            proposal.try_tip_vote(test_case.governing_token_supply, &governance_config,current_slot);

            // Assert
            assert_eq!(proposal.state,test_case.expected_state,"CASE: {:?}",test_case);

            if test_case.expected_state != ProposalState::Voting {
                assert_eq!(Some(current_slot),proposal.voting_completed_at)
            }
        }
    }

    #[test]
    pub fn test_finalize_vote_with_expired_voting_time_error() {
        // Arrange
        let mut proposal = create_test_proposal();
        proposal.state = ProposalState::Voting;
        let governance_config = create_test_governance_config();

        let current_slot = proposal.voting_at.unwrap() + governance_config.max_voting_time;

        // Act
        let err = proposal
            .finalize_vote(100, &governance_config, current_slot)
            .err()
            .unwrap();

        // Assert
        assert_eq!(err, GovernanceError::CannotFinalizeVotingInProgress.into());
    }

    #[test]
    pub fn test_finalize_vote_after_voting_time() {
        // Arrange
        let mut proposal = create_test_proposal();
        proposal.state = ProposalState::Voting;
        let governance_config = create_test_governance_config();

        let current_slot = proposal.voting_at.unwrap() + governance_config.max_voting_time + 1;

        // Act
        let result = proposal.finalize_vote(100, &governance_config, current_slot);

        // Assert
        assert_eq!(result, Ok(()));
    }

    #[test]
    pub fn test_assert_can_vote_with_expired_voting_time_error() {
        // Arrange
        let mut proposal = create_test_proposal();
        proposal.state = ProposalState::Voting;
        let governance_config = create_test_governance_config();

        let current_slot = proposal.voting_at.unwrap() + governance_config.max_voting_time + 1;

        // Act
        let err = proposal
            .assert_can_cast_vote(&governance_config, current_slot)
            .err()
            .unwrap();

        // Assert
        assert_eq!(err, GovernanceError::ProposalVotingTimeExpired.into());
    }

    #[test]
    pub fn test_assert_can_vote_within_voting_time() {
        // Arrange
        let mut proposal = create_test_proposal();
        proposal.state = ProposalState::Voting;
        let governance_config = create_test_governance_config();

        let current_slot = proposal.voting_at.unwrap() + governance_config.max_voting_time;

        // Act
        let result = proposal.assert_can_cast_vote(&governance_config, current_slot);

        // Assert
        assert_eq!(result, Ok(()));
    }
}
