//! State enumerations

use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};

/// Defines all Governance accounts types
#[repr(C)]
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub enum GovernanceAccountType {
    /// Default uninitialized account state
    Uninitialized,

    /// Top level aggregation for governances with Community Token (and optional Council Token)
    Realm,

    /// Token Owner Record for given governing token owner within a Realm
    TokenOwnerRecord,

    /// Generic Account Governance account
    AccountGovernance,

    /// Program Governance account
    ProgramGovernance,

    /// Proposal account for Governance account. A single Governance account can have multiple Proposal accounts
    Proposal,

    /// Proposal Signatory account
    SignatoryRecord,

    /// Vote record account for a given Proposal.  Proposal can have 0..n voting records
    VoteRecord,

    /// ProposalInstruction account which holds an instruction to execute for Proposal
    ProposalInstruction,

    /// Mint Governance account
    MintGovernance,

    /// Token Governance account
    TokenGovernance,
}

impl Default for GovernanceAccountType {
    fn default() -> Self {
        GovernanceAccountType::Uninitialized
    }
}

/// Vote  with number of votes
#[repr(C)]
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub enum VoteWeight {
    /// Yes vote
    Yes(u64),

    /// No vote
    No(u64),
}

/// What state a Proposal is in
#[repr(C)]
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub enum ProposalState {
    /// Draft - Proposal enters Draft state when it's created
    Draft,

    /// SigningOff - The Proposal is being signed off by Signatories
    /// Proposal enters the state when first Signatory Sings and leaves it when last Signatory signs
    SigningOff,

    /// Taking votes
    Voting,

    /// Voting ended with success
    Succeeded,

    /// Voting completed and now instructions are being execute. Proposal enter this state when first instruction is executed and leaves when the last instruction is executed
    Executing,

    /// Completed
    Completed,

    /// Cancelled
    Cancelled,

    /// Defeated
    Defeated,
}

impl Default for ProposalState {
    fn default() -> Self {
        ProposalState::Draft
    }
}
