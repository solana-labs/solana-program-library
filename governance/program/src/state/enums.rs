//! State enumerations

/// Defines all Governance accounts types
#[repr(C)]
#[derive(Clone, Debug, PartialEq)]
pub enum GovernanceAccountType {
    /// Default uninitialized account state
    Uninitialized,

    /// Top level aggregation for governances with Community Token (and optional Council Token)
    Realm,

    /// Voter record for each voter and given governing token type within a Realm
    VoterRecord,

    /// Program Governance account
    ProgramGovernance,

    /// Proposal account for Governance account. A single Governance account can have multiple Proposal accounts
    Proposal,

    /// Vote record account for a given Proposal.  Proposal can have 0..n voting records
    ProposalVoteRecord,

    /// Single Signer Instruction account which holds an instruction to execute for Proposal
    SingleSignerInstruction,
}

impl Default for GovernanceAccountType {
    fn default() -> Self {
        GovernanceAccountType::Uninitialized
    }
}

/// Vote  with number of votes
#[repr(C)]
#[derive(Clone, Debug, PartialEq)]
pub enum VoteWeight {
    /// Yes vote
    Yes(u64),

    /// No vote
    No(u64),
}

/// Governing Token type
#[repr(C)]
#[derive(Clone)]
pub enum GoverningTokenType {
    /// Community token
    Community,
    /// Council token
    Council,
}

/// What state a Proposal is in
#[repr(C)]
#[derive(Clone, Debug, PartialEq)]
pub enum ProposalState {
    /// Draft - Proposal enters Draft state when it's created
    Draft,

    /// Signing - The Proposal is being signed by Signatories. Proposal enters the state when first Signatory Sings and leaves it when last Signatory signs
    Signing,

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
