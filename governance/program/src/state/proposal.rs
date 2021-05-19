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
