//! Proposal  Account

use solana_program::{epoch_schedule::Slot, pubkey::Pubkey};

use super::enums::{GovernanceAccountType, GoverningTokenType, ProposalState};
use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};

/// Governance Proposal
#[repr(C)]
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub struct Proposal {
    /// Governance account type
    pub account_type: GovernanceAccountType,

    /// Governance account the Proposal belongs to
    pub governance: Pubkey,

    /// Mint that creates signatory tokens of this Proposal
    /// If there are outstanding signatory tokens, then cannot leave draft state. Signatories must burn tokens (ie agree
    /// to move instruction to voting state) and bring mint to net 0 tokens outstanding. Each signatory gets 1 (serves as flag)
    pub signatory_mint: Pubkey,

    /// Admin ownership mint. One token is minted, can be used to grant admin status to a new person
    pub admin_mint: Pubkey,

    /// Indicates which Governing Token is used to vote on the Proposal
    /// Whether the general Community token owners or the Council tokens owners vote on this Proposal
    pub voting_token_type: GoverningTokenType,

    /// Current state of the proposal
    pub state: ProposalState,

    /// Total signatory tokens minted, for use comparing to supply remaining during draft period
    pub total_signatory_tokens_minted: u64,

    /// Link to proposal's description
    pub description_link: String,

    /// Proposal name
    pub name: String,

    /// When the Proposal ended voting - this will also be when the set was defeated or began executing naturally
    pub voting_ended_at: Option<Slot>,

    /// When the Proposal began voting
    pub voting_began_at: Option<Slot>,

    /// when the Proposal entered draft state
    pub created_at: Option<Slot>,

    /// when the Proposal entered completed state, also when execution ended naturally.
    pub completed_at: Option<Slot>,

    /// when the Proposal entered deleted state
    pub deleted_at: Option<Slot>,

    /// The number of the instructions already executed
    pub number_of_executed_instructions: u8,

    /// The number of instructions included in the proposal
    pub number_of_instructions: u8,

    /// Array of pubkeys pointing at Proposal instructions, up to 5
    pub instruction: Vec<Pubkey>,
}

#[cfg(test)]
mod test {

    use {super::*, proptest::prelude::*};

    fn create_test_proposal() -> Proposal {
        Proposal {
            account_type: GovernanceAccountType::VoterRecord,
            governance: Pubkey::new_unique(),
            state: ProposalState::Draft,

            description_link: "This is my description".to_string(),
            name: "This is my name".to_string(),

            number_of_executed_instructions: 10,
            number_of_instructions: 10,
            signatory_mint: Pubkey::new_unique(),
            admin_mint: Pubkey::new_unique(),
            voting_token_type: GoverningTokenType::Community,
            total_signatory_tokens_minted: 1,
            voting_ended_at: Some(1),
            voting_began_at: Some(1),
            created_at: Some(1),
            completed_at: Some(1),
            deleted_at: Some(1),
            instruction: vec![],
        }
    }

    fn editable_signatory_states() -> impl Strategy<Value = ProposalState> {
        prop_oneof![Just(ProposalState::Draft), Just(ProposalState::SigningOff),]
    }

    proptest! {
        #[test]
        fn test_assert_can_add_signatory(state in editable_signatory_states()) {

            let mut proposal = create_test_proposal();
            proposal.state = state;
            proposal.assert_can_add_signatory().unwrap();

        }
        #[test]
        fn test_assert_can_remove_signatory(state in editable_signatory_states()) {

            let mut proposal = create_test_proposal();
            proposal.state = state;
            proposal.assert_can_add_signatory().unwrap();

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
        ]
    }

    proptest! {
        #[test]
            fn test_assert_can_add_signatory_with_invalid_state_error(state in none_editable_signatory_states()) {
                // Arrange
                let mut proposal = create_test_proposal();
                proposal.state = state;

                // Act
                let err = proposal.assert_can_add_signatory().err().unwrap();

                // Assert
                assert_eq!(err, GovernanceError::InvalidStateCannotAddSignatory.into());
        }
        #[test]
        fn test_assert_can_remove_signatory_with_state_error(state in none_editable_signatory_states()) {
            // Arrange
            let mut proposal = create_test_proposal();
            proposal.state = state;

            // Act
            let err = proposal.assert_can_remove_signatory().err().unwrap();

            // Assert
            assert_eq!(err, GovernanceError::InvalidStateCannotRemoveSignatory.into());
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

    #[test]
    fn test_assert_can_sign_off_with_proposal_without_signatories_error() {
        // Arrange
        let mut proposal = create_test_proposal();
        proposal.signatories_count = 0;

        // Act
        let err = proposal.assert_can_sign_off().err().unwrap();

        // Assert
        assert_eq!(err, GovernanceError::ProposalHasNoSignatories.into());
    }
}
