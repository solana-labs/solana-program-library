//! Proposal  Account

use solana_program::{
    account_info::AccountInfo, epoch_schedule::Slot, program_error::ProgramError,
    program_pack::IsInitialized, pubkey::Pubkey,
};

use crate::{
    error::GovernanceError,
    id,
    tools::account::{deserialize_account, AccountMaxSize},
    PROGRAM_AUTHORITY_SEED,
};

use crate::state::enums::{GovernanceAccountType, ProposalState};
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

    /// Link to proposal's description
    pub description_link: String,

    /// Proposal name
    pub name: String,

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
        Some(self.name.len() + self.description_link.len() + 163)
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
        if !(self.state == ProposalState::Draft || self.state == ProposalState::SigningOff) {
            return Err(GovernanceError::InvalidStateCannotEditSignatories.into());
        }

        Ok(())
    }

    /// Checks if Proposal can be singed off
    pub fn assert_can_sign_off(&self) -> Result<(), ProgramError> {
        if !(self.state == ProposalState::Draft || self.state == ProposalState::SigningOff) {
            return Err(GovernanceError::InvalidStateCannotSignOff.into());
        }

        Ok(())
    }
}

/// Deserializes Proposal account and checks owner program
pub fn deserialize_proposal_raw(proposal_info: &AccountInfo) -> Result<Proposal, ProgramError> {
    deserialize_account::<Proposal>(proposal_info, &id())
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
    proposal_index_bytes: &'a [u8],
) -> Pubkey {
    Pubkey::find_program_address(
        &get_proposal_address_seeds(governance, governing_token_mint, &proposal_index_bytes),
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
        }
    }

    #[test]
    fn test_max_size() {
        let proposal = create_test_proposal();
        let size = proposal.try_to_vec().unwrap().len();

        assert_eq!(proposal.get_max_size(), Some(size));
    }

    fn editable_signatory_states() -> impl Strategy<Value = ProposalState> {
        prop_oneof![Just(ProposalState::Draft), Just(ProposalState::SigningOff),]
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
}
