//! Proposal  Account

use solana_program::clock::{Slot, UnixTimestamp};

use solana_program::{
    account_info::AccountInfo, program_error::ProgramError, program_pack::IsInitialized,
    pubkey::Pubkey,
};

use crate::{
    error::GovernanceError,
    state::{
        enums::{
            GovernanceAccountType, InstructionExecutionFlags, InstructionExecutionStatus,
            MintMaxVoteWeightSource, ProposalState, VoteThresholdPercentage,
        },
        governance::GovernanceConfig,
        proposal_instruction::ProposalInstruction,
        realm::Realm,
    },
    tools::account::{get_account_data, AccountMaxSize},
    PROGRAM_AUTHORITY_SEED,
};
use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};

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

    /// The number of Yes votes
    pub yes_votes_count: u64,

    /// The number of No votes
    pub no_votes_count: u64,

    /// The number of the instructions already executed
    pub instructions_executed_count: u16,

    /// The number of instructions included in the proposal
    pub instructions_count: u16,

    /// The index of the the next instruction to be added
    pub instructions_next_index: u16,

    /// When the Proposal was created and entered Draft state
    pub draft_at: UnixTimestamp,

    /// When Signatories started signing off the Proposal
    pub signing_off_at: Option<UnixTimestamp>,

    /// When the Proposal began voting as UnixTimestamp
    pub voting_at: Option<UnixTimestamp>,

    /// When the Proposal began voting as Slot
    /// Note: The slot is not currently used but the exact slot is going to be required to support snapshot based vote weights
    pub voting_at_slot: Option<Slot>,

    /// When the Proposal ended voting and entered either Succeeded or Defeated
    pub voting_completed_at: Option<UnixTimestamp>,

    /// When the Proposal entered Executing state
    pub executing_at: Option<UnixTimestamp>,

    /// When the Proposal entered final state Completed or Cancelled and was closed
    pub closed_at: Option<UnixTimestamp>,

    /// Instruction execution flag for ordered and transactional instructions
    /// Note: This field is not used in the current version
    pub execution_flags: InstructionExecutionFlags,

    /// The max vote weight for the Governing Token mint at the time Proposal was decided
    /// It's used to show correct vote results for historical proposals in cases when the mint supply or max weight source changed
    /// after vote was completed.
    pub max_vote_weight: Option<u64>,

    /// The vote threshold percentage at the time Proposal was decided
    /// It's used to show correct vote results for historical proposals in cases when the threshold
    /// was changed for governance config after vote was completed.
    pub vote_threshold_percentage: Option<VoteThresholdPercentage>,

    /// Proposal name
    pub name: String,

    /// Link to proposal's description
    pub description_link: String,
}

impl AccountMaxSize for Proposal {
    fn get_max_size(&self) -> Option<usize> {
        Some(self.name.len() + self.description_link.len() + 205)
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
            | ProposalState::ExecutingWithErrors
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
        current_unix_timestamp: UnixTimestamp,
    ) -> Result<(), ProgramError> {
        self.assert_is_voting_state()
            .map_err(|_| GovernanceError::InvalidStateCannotVote)?;

        // Check if we are still within the configured max_voting_time period
        if self
            .voting_at
            .unwrap()
            .checked_add(config.max_voting_time as i64)
            .unwrap()
            < current_unix_timestamp
        {
            return Err(GovernanceError::ProposalVotingTimeExpired.into());
        }

        Ok(())
    }

    /// Checks if Proposal can be finalized
    pub fn assert_can_finalize_vote(
        &self,
        config: &GovernanceConfig,
        current_unix_timestamp: UnixTimestamp,
    ) -> Result<(), ProgramError> {
        self.assert_is_voting_state()
            .map_err(|_| GovernanceError::InvalidStateCannotFinalize)?;

        // Check if we passed the configured max_voting_time period yet
        if self
            .voting_at
            .unwrap()
            .checked_add(config.max_voting_time as i64)
            .unwrap()
            >= current_unix_timestamp
        {
            return Err(GovernanceError::CannotFinalizeVotingInProgress.into());
        }

        Ok(())
    }

    /// Finalizes vote by moving it to final state Succeeded or Defeated if max_voting_time has passed
    /// If Proposal is still within max_voting_time period then error is returned
    pub fn finalize_vote(
        &mut self,
        governing_token_mint_supply: u64,
        config: &GovernanceConfig,
        realm_data: &Realm,
        current_unix_timestamp: UnixTimestamp,
    ) -> Result<(), ProgramError> {
        self.assert_can_finalize_vote(config, current_unix_timestamp)?;

        let max_vote_weight = self.get_max_vote_weight(realm_data, governing_token_mint_supply)?;

        self.state = self.get_final_vote_state(max_vote_weight, config);
        self.voting_completed_at = Some(current_unix_timestamp);

        // Capture vote params to correctly display historical results
        self.max_vote_weight = Some(max_vote_weight);
        self.vote_threshold_percentage = Some(config.vote_threshold_percentage.clone());

        Ok(())
    }

    fn get_final_vote_state(
        &mut self,
        max_vote_weight: u64,
        config: &GovernanceConfig,
    ) -> ProposalState {
        let yes_vote_threshold_count =
            get_yes_vote_threshold_count(&config.vote_threshold_percentage, max_vote_weight)
                .unwrap();

        // Yes vote must be equal or above the required yes_vote_threshold_percentage and higher than No vote
        // The same number of Yes and No votes is a tie and resolved as Defeated
        // In other words  +1 vote as a tie breaker is required to Succeed
        if self.yes_votes_count >= yes_vote_threshold_count
            && self.yes_votes_count > self.no_votes_count
        {
            ProposalState::Succeeded
        } else {
            ProposalState::Defeated
        }
    }

    /// Calculates max vote weight for given mint supply and realm config
    fn get_max_vote_weight(
        &mut self,
        realm_data: &Realm,
        governing_token_mint_supply: u64,
    ) -> Result<u64, ProgramError> {
        // max vote weight fraction is only used for community mint
        if Some(self.governing_token_mint) == realm_data.config.council_mint {
            return Ok(governing_token_mint_supply);
        }

        match realm_data.config.community_mint_max_vote_weight_source {
            MintMaxVoteWeightSource::SupplyFraction(fraction) => {
                if fraction == MintMaxVoteWeightSource::SUPPLY_FRACTION_BASE {
                    return Ok(governing_token_mint_supply);
                }

                let max_vote_weight = (governing_token_mint_supply as u128)
                    .checked_mul(fraction as u128)
                    .unwrap()
                    .checked_div(MintMaxVoteWeightSource::SUPPLY_FRACTION_BASE as u128)
                    .unwrap() as u64;

                // When the fraction is used it's possible we can go over the calculated max_vote_weight
                // and we have to adjust it in case more votes have been cast
                let total_vote_count = self
                    .yes_votes_count
                    .checked_add(self.no_votes_count)
                    .unwrap();

                Ok(max_vote_weight.max(total_vote_count))
            }
            MintMaxVoteWeightSource::Absolute(_) => {
                Err(GovernanceError::VoteWeightSourceNotSupported.into())
            }
        }
    }

    /// Checks if vote can be tipped and automatically transitioned to Succeeded or Defeated state
    /// If the conditions are met the state is updated accordingly
    pub fn try_tip_vote(
        &mut self,
        governing_token_mint_supply: u64,
        config: &GovernanceConfig,
        realm_data: &Realm,
        current_unix_timestamp: UnixTimestamp,
    ) -> Result<bool, ProgramError> {
        let max_vote_weight = self.get_max_vote_weight(realm_data, governing_token_mint_supply)?;

        if let Some(tipped_state) = self.try_get_tipped_vote_state(max_vote_weight, config) {
            self.state = tipped_state;
            self.voting_completed_at = Some(current_unix_timestamp);

            // Capture vote params to correctly display historical results
            self.max_vote_weight = Some(max_vote_weight);
            self.vote_threshold_percentage = Some(config.vote_threshold_percentage.clone());

            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Checks if vote can be tipped and automatically transitioned to Succeeded or Defeated state
    /// If yes then Some(ProposalState) is returned and None otherwise
    #[allow(clippy::float_cmp)]
    pub fn try_get_tipped_vote_state(
        &self,
        max_vote_weight: u64,
        config: &GovernanceConfig,
    ) -> Option<ProposalState> {
        if self.yes_votes_count == max_vote_weight {
            return Some(ProposalState::Succeeded);
        }
        if self.no_votes_count == max_vote_weight {
            return Some(ProposalState::Defeated);
        }

        let yes_vote_threshold_count =
            get_yes_vote_threshold_count(&config.vote_threshold_percentage, max_vote_weight)
                .unwrap();

        if self.yes_votes_count >= yes_vote_threshold_count
            && self.yes_votes_count > (max_vote_weight - self.yes_votes_count)
        {
            return Some(ProposalState::Succeeded);
        } else if self.no_votes_count > (max_vote_weight - yes_vote_threshold_count)
            || self.no_votes_count >= (max_vote_weight - self.no_votes_count)
        {
            return Some(ProposalState::Defeated);
        }

        None
    }

    /// Checks if Proposal can be canceled in the given state
    pub fn assert_can_cancel(&self) -> Result<(), ProgramError> {
        match self.state {
            ProposalState::Draft | ProposalState::SigningOff | ProposalState::Voting => Ok(()),
            ProposalState::Executing
            | ProposalState::ExecutingWithErrors
            | ProposalState::Completed
            | ProposalState::Cancelled
            | ProposalState::Succeeded
            | ProposalState::Defeated => {
                Err(GovernanceError::InvalidStateCannotCancelProposal.into())
            }
        }
    }

    /// Checks if Instructions can be edited (inserted or removed) for the Proposal in the given state
    pub fn assert_can_edit_instructions(&self) -> Result<(), ProgramError> {
        self.assert_is_draft_state()
            .map_err(|_| GovernanceError::InvalidStateCannotEditInstructions.into())
    }

    /// Checks if Instructions can be executed for the Proposal in the given state
    pub fn assert_can_execute_instruction(
        &self,
        proposal_instruction_data: &ProposalInstruction,
        current_unix_timestamp: UnixTimestamp,
    ) -> Result<(), ProgramError> {
        match self.state {
            ProposalState::Succeeded
            | ProposalState::Executing
            | ProposalState::ExecutingWithErrors => {}
            ProposalState::Draft
            | ProposalState::SigningOff
            | ProposalState::Completed
            | ProposalState::Voting
            | ProposalState::Cancelled
            | ProposalState::Defeated => {
                return Err(GovernanceError::InvalidStateCannotExecuteInstruction.into())
            }
        }

        if self
            .voting_completed_at
            .unwrap()
            .checked_add(proposal_instruction_data.hold_up_time as i64)
            .unwrap()
            >= current_unix_timestamp
        {
            return Err(GovernanceError::CannotExecuteInstructionWithinHoldUpTime.into());
        }

        if proposal_instruction_data.executed_at.is_some() {
            return Err(GovernanceError::InstructionAlreadyExecuted.into());
        }

        Ok(())
    }

    /// Checks if the instruction can be flagged with error for the Proposal in the given state
    pub fn assert_can_flag_instruction_error(
        &self,
        proposal_instruction_data: &ProposalInstruction,
        current_unix_timestamp: UnixTimestamp,
    ) -> Result<(), ProgramError> {
        // Instruction can be flagged for error only when it's eligible for execution
        self.assert_can_execute_instruction(proposal_instruction_data, current_unix_timestamp)?;

        if proposal_instruction_data.execution_status == InstructionExecutionStatus::Error {
            return Err(GovernanceError::InstructionAlreadyFlaggedWithError.into());
        }

        Ok(())
    }
}

/// Converts threshold in percentages to actual vote count
fn get_yes_vote_threshold_count(
    vote_threshold_percentage: &VoteThresholdPercentage,
    max_vote_weight: u64,
) -> Result<u64, ProgramError> {
    let yes_vote_threshold_percentage = match vote_threshold_percentage {
        VoteThresholdPercentage::YesVote(yes_vote_threshold_percentage) => {
            *yes_vote_threshold_percentage
        }
        _ => {
            return Err(GovernanceError::VoteThresholdPercentageTypeNotSupported.into());
        }
    };

    let numerator = (yes_vote_threshold_percentage as u128)
        .checked_mul(max_vote_weight as u128)
        .unwrap();

    let mut yes_vote_threshold = numerator.checked_div(100).unwrap();

    if yes_vote_threshold * 100 < numerator {
        yes_vote_threshold += 1;
    }

    Ok(yes_vote_threshold as u64)
}

/// Deserializes Proposal account and checks owner program
pub fn get_proposal_data(
    program_id: &Pubkey,
    proposal_info: &AccountInfo,
) -> Result<Proposal, ProgramError> {
    get_account_data::<Proposal>(proposal_info, program_id)
}

/// Deserializes Proposal and validates it belongs to the given Governance and Governing Mint
pub fn get_proposal_data_for_governance_and_governing_mint(
    program_id: &Pubkey,
    proposal_info: &AccountInfo,
    governance: &Pubkey,
    governing_token_mint: &Pubkey,
) -> Result<Proposal, ProgramError> {
    let proposal_data = get_proposal_data_for_governance(program_id, proposal_info, governance)?;

    if proposal_data.governing_token_mint != *governing_token_mint {
        return Err(GovernanceError::InvalidGoverningMintForProposal.into());
    }

    Ok(proposal_data)
}

/// Deserializes Proposal and validates it belongs to the given Governance
pub fn get_proposal_data_for_governance(
    program_id: &Pubkey,
    proposal_info: &AccountInfo,
    governance: &Pubkey,
) -> Result<Proposal, ProgramError> {
    let proposal_data = get_proposal_data(program_id, proposal_info)?;

    if proposal_data.governance != *governance {
        return Err(GovernanceError::InvalidGovernanceForProposal.into());
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
        proposal_index_le_bytes,
    ]
}

/// Returns Proposal PDA address
pub fn get_proposal_address<'a>(
    program_id: &Pubkey,
    governance: &'a Pubkey,
    governing_token_mint: &'a Pubkey,
    proposal_index_le_bytes: &'a [u8],
) -> Pubkey {
    Pubkey::find_program_address(
        &get_proposal_address_seeds(governance, governing_token_mint, proposal_index_le_bytes),
        program_id,
    )
    .0
}

#[cfg(test)]
mod test {
    use crate::state::{
        enums::{MintMaxVoteWeightSource, VoteThresholdPercentage, VoteWeightSource},
        realm::RealmConfig,
    };

    use {super::*, proptest::prelude::*};

    fn create_test_proposal() -> Proposal {
        Proposal {
            account_type: GovernanceAccountType::TokenOwnerRecord,
            governance: Pubkey::new_unique(),
            governing_token_mint: Pubkey::new_unique(),
            max_vote_weight: Some(10),
            state: ProposalState::Draft,
            token_owner_record: Pubkey::new_unique(),
            signatories_count: 10,
            signatories_signed_off_count: 5,
            description_link: "This is my description".to_string(),
            name: "This is my name".to_string(),
            draft_at: 10,
            signing_off_at: Some(10),

            voting_at: Some(10),
            voting_at_slot: Some(500),

            voting_completed_at: Some(10),
            executing_at: Some(10),
            closed_at: Some(10),

            yes_votes_count: 0,
            no_votes_count: 0,

            execution_flags: InstructionExecutionFlags::Ordered,

            instructions_executed_count: 10,
            instructions_count: 10,
            instructions_next_index: 10,
            vote_threshold_percentage: Some(VoteThresholdPercentage::YesVote(100)),
        }
    }

    fn create_test_realm() -> Realm {
        Realm {
            account_type: GovernanceAccountType::Realm,
            community_mint: Pubkey::new_unique(),
            reserved: [0; 8],

            authority: Some(Pubkey::new_unique()),
            name: "test-realm".to_string(),
            config: RealmConfig {
                council_mint: Some(Pubkey::new_unique()),
                reserved: [0; 8],

                community_mint_max_vote_weight_source:
                    MintMaxVoteWeightSource::FULL_SUPPLY_FRACTION,
                min_community_tokens_to_create_governance: 10,
            },
        }
    }

    fn create_test_governance_config() -> GovernanceConfig {
        GovernanceConfig {
            min_community_tokens_to_create_proposal: 5,
            min_council_tokens_to_create_proposal: 1,
            min_instruction_hold_up_time: 10,
            max_voting_time: 5,
            vote_threshold_percentage: VoteThresholdPercentage::YesVote(60),
            vote_weight_source: VoteWeightSource::Deposit,
            proposal_cool_off_time: 0,
        }
    }

    #[test]
    fn test_max_size() {
        let proposal = create_test_proposal();
        let size = proposal.try_to_vec().unwrap().len();

        assert_eq!(proposal.get_max_size(), Some(size));
    }

    prop_compose! {
        fn vote_results()(governing_token_supply in 1..=u64::MAX)(
            governing_token_supply in Just(governing_token_supply),
            vote_count in 0..=governing_token_supply,
        ) -> (u64, u64) {
            (vote_count as u64, governing_token_supply as u64)
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
            Just(ProposalState::ExecutingWithErrors),
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
            Just(ProposalState::ExecutingWithErrors),
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
            Just(ProposalState::ExecutingWithErrors),
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
    pub struct VoteCastTestCase {
        name: &'static str,
        governing_token_supply: u64,
        vote_threshold_percentage: u8,
        yes_votes_count: u64,
        no_votes_count: u64,
        expected_tipped_state: ProposalState,
        expected_finalized_state: ProposalState,
    }

    fn vote_casting_test_cases() -> impl Strategy<Value = VoteCastTestCase> {
        prop_oneof![
            //  threshold < 50%
            Just(VoteCastTestCase {
                name: "45:10 @40 -- Nays can still outvote Yeahs",
                governing_token_supply: 100,
                vote_threshold_percentage: 40,
                yes_votes_count: 45,
                no_votes_count: 10,
                expected_tipped_state: ProposalState::Voting,
                expected_finalized_state: ProposalState::Succeeded,
            }),
            Just(VoteCastTestCase {
                name: "49:50 @40 -- In best case scenario it can be 50:50 tie and hence Defeated",
                governing_token_supply: 100,
                vote_threshold_percentage: 40,
                yes_votes_count: 49,
                no_votes_count: 50,
                expected_tipped_state: ProposalState::Defeated,
                expected_finalized_state: ProposalState::Defeated,
            }),
            Just(VoteCastTestCase {
                name: "40:40 @40 -- Still can go either way",
                governing_token_supply: 100,
                vote_threshold_percentage: 40,
                yes_votes_count: 40,
                no_votes_count: 40,
                expected_tipped_state: ProposalState::Voting,
                expected_finalized_state: ProposalState::Defeated,
            }),
            Just(VoteCastTestCase {
                name: "45:45 @40 -- Still can go either way",
                governing_token_supply: 100,
                vote_threshold_percentage: 40,
                yes_votes_count: 45,
                no_votes_count: 45,
                expected_tipped_state: ProposalState::Voting,
                expected_finalized_state: ProposalState::Defeated,
            }),
            Just(VoteCastTestCase {
                name: "50:10 @40 -- Nay sayers can still tie up",
                governing_token_supply: 100,
                vote_threshold_percentage: 40,
                yes_votes_count: 50,
                no_votes_count: 10,
                expected_tipped_state: ProposalState::Voting,
                expected_finalized_state: ProposalState::Succeeded,
            }),
            Just(VoteCastTestCase {
                name: "50:50 @40 -- It's a tie and hence Defeated",
                governing_token_supply: 100,
                vote_threshold_percentage: 40,
                yes_votes_count: 50,
                no_votes_count: 50,
                expected_tipped_state: ProposalState::Defeated,
                expected_finalized_state: ProposalState::Defeated,
            }),
            Just(VoteCastTestCase {
                name: "45:51 @ 40 -- Nays won",
                governing_token_supply: 100,
                vote_threshold_percentage: 40,
                yes_votes_count: 45,
                no_votes_count: 51,
                expected_tipped_state: ProposalState::Defeated,
                expected_finalized_state: ProposalState::Defeated,
            }),
            Just(VoteCastTestCase {
                name: "40:55 @ 40 -- Nays won",
                governing_token_supply: 100,
                vote_threshold_percentage: 40,
                yes_votes_count: 40,
                no_votes_count: 55,
                expected_tipped_state: ProposalState::Defeated,
                expected_finalized_state: ProposalState::Defeated,
            }),
            // threshold == 50%
            Just(VoteCastTestCase {
                name: "50:10 @50 -- +1 tie breaker required to tip",
                governing_token_supply: 100,
                vote_threshold_percentage: 50,
                yes_votes_count: 50,
                no_votes_count: 10,
                expected_tipped_state: ProposalState::Voting,
                expected_finalized_state: ProposalState::Succeeded,
            }),
            Just(VoteCastTestCase {
                name: "10:50 @50 -- +1 tie breaker vote not possible any longer",
                governing_token_supply: 100,
                vote_threshold_percentage: 50,
                yes_votes_count: 10,
                no_votes_count: 50,
                expected_tipped_state: ProposalState::Defeated,
                expected_finalized_state: ProposalState::Defeated,
            }),
            Just(VoteCastTestCase {
                name: "50:50 @50 -- +1 tie breaker vote not possible any longer",
                governing_token_supply: 100,
                vote_threshold_percentage: 50,
                yes_votes_count: 50,
                no_votes_count: 50,
                expected_tipped_state: ProposalState::Defeated,
                expected_finalized_state: ProposalState::Defeated,
            }),
            Just(VoteCastTestCase {
                name: "51:10 @ 50 -- Nay sayers can't outvote any longer",
                governing_token_supply: 100,
                vote_threshold_percentage: 50,
                yes_votes_count: 51,
                no_votes_count: 10,
                expected_tipped_state: ProposalState::Succeeded,
                expected_finalized_state: ProposalState::Succeeded,
            }),
            Just(VoteCastTestCase {
                name: "10:51 @ 50 -- Nays won",
                governing_token_supply: 100,
                vote_threshold_percentage: 50,
                yes_votes_count: 10,
                no_votes_count: 51,
                expected_tipped_state: ProposalState::Defeated,
                expected_finalized_state: ProposalState::Defeated,
            }),
            // threshold > 50%
            Just(VoteCastTestCase {
                name: "10:10 @ 60 -- Can still go either way",
                governing_token_supply: 100,
                vote_threshold_percentage: 60,
                yes_votes_count: 10,
                no_votes_count: 10,
                expected_tipped_state: ProposalState::Voting,
                expected_finalized_state: ProposalState::Defeated,
            }),
            Just(VoteCastTestCase {
                name: "55:10 @ 60 -- Can still go either way",
                governing_token_supply: 100,
                vote_threshold_percentage: 60,
                yes_votes_count: 55,
                no_votes_count: 10,
                expected_tipped_state: ProposalState::Voting,
                expected_finalized_state: ProposalState::Defeated,
            }),
            Just(VoteCastTestCase {
                name: "60:10 @ 60 -- Yeah reached the required threshold",
                governing_token_supply: 100,
                vote_threshold_percentage: 60,
                yes_votes_count: 60,
                no_votes_count: 10,
                expected_tipped_state: ProposalState::Succeeded,
                expected_finalized_state: ProposalState::Succeeded,
            }),
            Just(VoteCastTestCase {
                name: "61:10 @ 60 -- Yeah won",
                governing_token_supply: 100,
                vote_threshold_percentage: 60,
                yes_votes_count: 61,
                no_votes_count: 10,
                expected_tipped_state: ProposalState::Succeeded,
                expected_finalized_state: ProposalState::Succeeded,
            }),
            Just(VoteCastTestCase {
                name: "10:40 @ 60 -- Yeah can still outvote Nay",
                governing_token_supply: 100,
                vote_threshold_percentage: 60,
                yes_votes_count: 10,
                no_votes_count: 40,
                expected_tipped_state: ProposalState::Voting,
                expected_finalized_state: ProposalState::Defeated,
            }),
            Just(VoteCastTestCase {
                name: "60:40 @ 60 -- Yeah won",
                governing_token_supply: 100,
                vote_threshold_percentage: 60,
                yes_votes_count: 60,
                no_votes_count: 40,
                expected_tipped_state: ProposalState::Succeeded,
                expected_finalized_state: ProposalState::Succeeded,
            }),
            Just(VoteCastTestCase {
                name: "10:41 @ 60 -- Aye can't outvote Nay any longer",
                governing_token_supply: 100,
                vote_threshold_percentage: 60,
                yes_votes_count: 10,
                no_votes_count: 41,
                expected_tipped_state: ProposalState::Defeated,
                expected_finalized_state: ProposalState::Defeated,
            }),
            Just(VoteCastTestCase {
                name: "100:0",
                governing_token_supply: 100,
                vote_threshold_percentage: 100,
                yes_votes_count: 100,
                no_votes_count: 0,
                expected_tipped_state: ProposalState::Succeeded,
                expected_finalized_state: ProposalState::Succeeded,
            }),
            Just(VoteCastTestCase {
                name: "0:100",
                governing_token_supply: 100,
                vote_threshold_percentage: 100,
                yes_votes_count: 0,
                no_votes_count: 100,
                expected_tipped_state: ProposalState::Defeated,
                expected_finalized_state: ProposalState::Defeated,
            }),
        ]
    }

    proptest! {
        #[test]
        fn test_try_tip_vote(test_case in vote_casting_test_cases()) {
            // Arrange
            let mut proposal = create_test_proposal();
            proposal.yes_votes_count = test_case.yes_votes_count;
            proposal.no_votes_count = test_case.no_votes_count;
            proposal.state = ProposalState::Voting;

            let mut governance_config = create_test_governance_config();
            governance_config.vote_threshold_percentage =  VoteThresholdPercentage::YesVote(test_case.vote_threshold_percentage);

            let current_timestamp = 15_i64;

            let realm = create_test_realm();

            // Act
            proposal.try_tip_vote(test_case.governing_token_supply, &governance_config,&realm,current_timestamp).unwrap();

            // Assert
            assert_eq!(proposal.state,test_case.expected_tipped_state,"CASE: {:?}",test_case);

            if test_case.expected_tipped_state != ProposalState::Voting {
                assert_eq!(Some(current_timestamp),proposal.voting_completed_at)
            }
        }

        #[test]
        fn test_finalize_vote(test_case in vote_casting_test_cases()) {
            // Arrange
            let mut proposal = create_test_proposal();
            proposal.yes_votes_count = test_case.yes_votes_count;
            proposal.no_votes_count = test_case.no_votes_count;
            proposal.state = ProposalState::Voting;

            let mut governance_config = create_test_governance_config();
            governance_config.vote_threshold_percentage = VoteThresholdPercentage::YesVote(test_case.vote_threshold_percentage);

            let current_timestamp = 16_i64;

            let realm = create_test_realm();

            // Act
            proposal.finalize_vote(test_case.governing_token_supply, &governance_config,&realm,current_timestamp).unwrap();

            // Assert
            assert_eq!(proposal.state,test_case.expected_finalized_state,"CASE: {:?}",test_case);
            assert_eq!(Some(current_timestamp),proposal.voting_completed_at);

        }
    }

    prop_compose! {
        fn full_vote_results()(governing_token_supply in 1..=u64::MAX, yes_vote_threshold in 1..100)(
            governing_token_supply in Just(governing_token_supply),
            yes_vote_threshold in Just(yes_vote_threshold),

            yes_votes_count in 0..=governing_token_supply,
            no_votes_count in 0..=governing_token_supply,

        ) -> (u64, u64, u64, u8) {
            (yes_votes_count as u64, no_votes_count as u64, governing_token_supply as u64,yes_vote_threshold as u8)
        }
    }

    proptest! {
        #[test]
        fn test_try_tip_vote_with_full_vote_results(
            (yes_votes_count, no_votes_count, governing_token_supply, yes_vote_threshold_percentage) in full_vote_results(),

        ) {
            // Arrange

            let mut proposal = create_test_proposal();
            proposal.yes_votes_count = yes_votes_count;
            proposal.no_votes_count =no_votes_count.min(governing_token_supply-yes_votes_count);
            proposal.state = ProposalState::Voting;


            let mut governance_config = create_test_governance_config();
            let  yes_vote_threshold_percentage = VoteThresholdPercentage::YesVote(yes_vote_threshold_percentage);
            governance_config.vote_threshold_percentage = yes_vote_threshold_percentage.clone();

            let current_timestamp = 15_i64;

            let realm = create_test_realm();

            // Act
            proposal.try_tip_vote(governing_token_supply, &governance_config,&realm, current_timestamp).unwrap();

            // Assert
            let yes_vote_threshold_count = get_yes_vote_threshold_count(&yes_vote_threshold_percentage,governing_token_supply).unwrap();

            if yes_votes_count >= yes_vote_threshold_count && yes_votes_count > (governing_token_supply - yes_votes_count)
            {
                assert_eq!(proposal.state,ProposalState::Succeeded);
            } else if proposal.no_votes_count > (governing_token_supply - yes_vote_threshold_count)
                || proposal.no_votes_count >= (governing_token_supply - proposal.no_votes_count ) {
                assert_eq!(proposal.state,ProposalState::Defeated);
            } else {
                assert_eq!(proposal.state,ProposalState::Voting);
            }
        }
    }

    proptest! {
        #[test]
        fn test_finalize_vote_with_full_vote_results(
            (yes_votes_count, no_votes_count, governing_token_supply, yes_vote_threshold_percentage) in full_vote_results(),

        ) {
            // Arrange
            let mut proposal = create_test_proposal();
            proposal.yes_votes_count = yes_votes_count;
            proposal.no_votes_count = no_votes_count.min(governing_token_supply-yes_votes_count);
            proposal.state = ProposalState::Voting;


            let mut governance_config = create_test_governance_config();
            let  yes_vote_threshold_percentage = VoteThresholdPercentage::YesVote(yes_vote_threshold_percentage);

            governance_config.vote_threshold_percentage = yes_vote_threshold_percentage.clone();

            let current_timestamp = 16_i64;

            let realm = create_test_realm();

            // Act
            proposal.finalize_vote(governing_token_supply, &governance_config,&realm,current_timestamp).unwrap();

            // Assert
            let yes_vote_threshold_count = get_yes_vote_threshold_count(&yes_vote_threshold_percentage,governing_token_supply).unwrap();

            if yes_votes_count >= yes_vote_threshold_count &&  yes_votes_count > proposal.no_votes_count
            {
                assert_eq!(proposal.state,ProposalState::Succeeded);
            } else {
                assert_eq!(proposal.state,ProposalState::Defeated);
            }
        }
    }

    #[test]
    fn test_try_tip_vote_with_reduced_community_mint_max_vote_weight() {
        // Arrange
        let mut proposal = create_test_proposal();
        proposal.yes_votes_count = 60;
        proposal.no_votes_count = 10;
        proposal.state = ProposalState::Voting;

        let mut governance_config = create_test_governance_config();
        governance_config.vote_threshold_percentage = VoteThresholdPercentage::YesVote(60);

        let current_timestamp = 15_i64;

        let community_token_supply = 200;

        let mut realm = create_test_realm();

        // reduce max vote weight to 100
        realm.config.community_mint_max_vote_weight_source =
            MintMaxVoteWeightSource::SupplyFraction(
                MintMaxVoteWeightSource::SUPPLY_FRACTION_BASE / 2,
            );

        // Act
        proposal
            .try_tip_vote(
                community_token_supply,
                &governance_config,
                &realm,
                current_timestamp,
            )
            .unwrap();

        // Assert
        assert_eq!(proposal.state, ProposalState::Succeeded);
        assert_eq!(proposal.max_vote_weight, Some(100));
    }

    #[test]
    fn test_try_tip_vote_with_reduced_community_mint_max_vote_weight_and_vote_overflow() {
        // Arrange
        let mut proposal = create_test_proposal();

        proposal.no_votes_count = 10;
        proposal.state = ProposalState::Voting;

        let mut governance_config = create_test_governance_config();
        governance_config.vote_threshold_percentage = VoteThresholdPercentage::YesVote(60);

        let current_timestamp = 15_i64;

        let community_token_supply = 200;

        let mut realm = create_test_realm();

        // reduce max vote weight to 100
        realm.config.community_mint_max_vote_weight_source =
            MintMaxVoteWeightSource::SupplyFraction(
                MintMaxVoteWeightSource::SUPPLY_FRACTION_BASE / 2,
            );

        // vote above reduced supply
        proposal.yes_votes_count = 120;

        // Act
        proposal
            .try_tip_vote(
                community_token_supply,
                &governance_config,
                &realm,
                current_timestamp,
            )
            .unwrap();

        // Assert
        assert_eq!(proposal.state, ProposalState::Succeeded);
        assert_eq!(proposal.max_vote_weight, Some(130));
    }

    #[test]
    fn test_try_tip_vote_for_council_vote_with_reduced_community_mint_max_vote_weight() {
        // Arrange
        let mut proposal = create_test_proposal();
        proposal.yes_votes_count = 60;
        proposal.no_votes_count = 10;
        proposal.state = ProposalState::Voting;

        let mut governance_config = create_test_governance_config();
        governance_config.vote_threshold_percentage = VoteThresholdPercentage::YesVote(60);

        let current_timestamp = 15_i64;

        let community_token_supply = 200;

        let mut realm = create_test_realm();
        realm.config.community_mint_max_vote_weight_source =
            MintMaxVoteWeightSource::SupplyFraction(
                MintMaxVoteWeightSource::SUPPLY_FRACTION_BASE / 2,
            );
        realm.config.council_mint = Some(proposal.governing_token_mint);

        // Act
        proposal
            .try_tip_vote(
                community_token_supply,
                &governance_config,
                &realm,
                current_timestamp,
            )
            .unwrap();

        // Assert
        assert_eq!(proposal.state, ProposalState::Voting);
    }

    #[test]
    fn test_finalize_vote_with_reduced_community_mint_max_vote_weight() {
        // Arrange
        let mut proposal = create_test_proposal();
        proposal.yes_votes_count = 60;
        proposal.no_votes_count = 10;
        proposal.state = ProposalState::Voting;

        let mut governance_config = create_test_governance_config();
        governance_config.vote_threshold_percentage = VoteThresholdPercentage::YesVote(60);

        let current_timestamp = 16_i64;
        let community_token_supply = 200;

        let mut realm = create_test_realm();

        // reduce max vote weight to 100
        realm.config.community_mint_max_vote_weight_source =
            MintMaxVoteWeightSource::SupplyFraction(
                MintMaxVoteWeightSource::SUPPLY_FRACTION_BASE / 2,
            );

        // Act
        proposal
            .finalize_vote(
                community_token_supply,
                &governance_config,
                &realm,
                current_timestamp,
            )
            .unwrap();

        // Assert
        assert_eq!(proposal.state, ProposalState::Succeeded);
        assert_eq!(proposal.max_vote_weight, Some(100));
    }

    #[test]
    fn test_finalize_vote_with_reduced_community_mint_max_vote_weight_and_vote_overflow() {
        // Arrange
        let mut proposal = create_test_proposal();
        proposal.yes_votes_count = 60;
        proposal.no_votes_count = 10;
        proposal.state = ProposalState::Voting;

        let mut governance_config = create_test_governance_config();
        governance_config.vote_threshold_percentage = VoteThresholdPercentage::YesVote(60);

        let current_timestamp = 16_i64;
        let community_token_supply = 200;

        let mut realm = create_test_realm();

        // reduce max vote weight to 100
        realm.config.community_mint_max_vote_weight_source =
            MintMaxVoteWeightSource::SupplyFraction(
                MintMaxVoteWeightSource::SUPPLY_FRACTION_BASE / 2,
            );

        // vote above reduced supply
        proposal.yes_votes_count = 120;

        // Act
        proposal
            .finalize_vote(
                community_token_supply,
                &governance_config,
                &realm,
                current_timestamp,
            )
            .unwrap();

        // Assert
        assert_eq!(proposal.state, ProposalState::Succeeded);
        assert_eq!(proposal.max_vote_weight, Some(130));
    }

    #[test]
    pub fn test_finalize_vote_with_expired_voting_time_error() {
        // Arrange
        let mut proposal = create_test_proposal();
        proposal.state = ProposalState::Voting;
        let governance_config = create_test_governance_config();

        let current_timestamp =
            proposal.voting_at.unwrap() + governance_config.max_voting_time as i64;

        let realm = create_test_realm();

        // Act
        let err = proposal
            .finalize_vote(100, &governance_config, &realm, current_timestamp)
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

        let current_timestamp =
            proposal.voting_at.unwrap() + governance_config.max_voting_time as i64 + 1;

        let realm = create_test_realm();
        // Act
        let result = proposal.finalize_vote(100, &governance_config, &realm, current_timestamp);

        // Assert
        assert_eq!(result, Ok(()));
    }

    #[test]
    pub fn test_assert_can_vote_with_expired_voting_time_error() {
        // Arrange
        let mut proposal = create_test_proposal();
        proposal.state = ProposalState::Voting;
        let governance_config = create_test_governance_config();

        let current_timestamp =
            proposal.voting_at.unwrap() + governance_config.max_voting_time as i64 + 1;

        // Act
        let err = proposal
            .assert_can_cast_vote(&governance_config, current_timestamp)
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

        let current_timestamp =
            proposal.voting_at.unwrap() + governance_config.max_voting_time as i64;

        // Act
        let result = proposal.assert_can_cast_vote(&governance_config, current_timestamp);

        // Assert
        assert_eq!(result, Ok(()));
    }
}
