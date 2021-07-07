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

/// The type of the vote threshold percentage used to resolve a vote on a Proposal
#[repr(C)]
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub enum VoteThresholdPercentage {
    /// Voting threshold of Yes votes in % required to tip the vote
    /// It's the percentage of tokens out of the entire pool of governance tokens eligible to vote
    /// Note: If the threshold is below or equal to 50% then an even split of votes ex: 50:50 or 40:40 is always resolved as Defeated
    /// In other words a '+1 vote' tie breaker is always required to have a successful vote
    YesVote(u8),

    /// The minimum number of votes in % out of the entire pool of governance tokens eligible to vote
    /// which must be cast for the vote to be valid
    /// Once the quorum is achieved a simple majority (50%+1) of Yes votes is required for the vote to succeed
    /// Note: Quorum is not implemented in the current version
    Quorum(u8),
}

/// The source of voter weights used to vote on proposals
#[repr(C)]
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub enum VoteWeightSource {
    /// Governing token deposits into the Realm are used as voter weights
    Deposit,
    /// Governing token account snapshots as of the time a proposal entered voting state are used as voter weights
    /// Note: Snapshot source is not supported in the current version
    /// Support for account snapshots are required in solana and/or arweave as a prerequisite
    Snapshot,
}

/// The status of instruction execution
#[repr(C)]
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub enum InstructionExecutionStatus {
    /// Instruction was not executed yet
    None,

    /// Instruction was executed successfully
    Success,

    /// Instruction execution failed
    /// Note: Error status is not supported yet because when CPI call fails it always terminates parent instruction
    /// We either have to make it possible to change that behavior or add an instruction to manually set the status
    Error,
}

/// Instruction execution flags defining how instructions are executed for a Proposal
#[repr(C)]
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub enum InstructionExecutionFlags {
    /// No execution flags are specified
    /// Instructions can be executed individually, in any order, as soon as they hold_up time expires
    None,

    /// Instructions are executed in a specific order
    /// Note: Ordered execution is not supported in the current version
    /// The implementation requires another account type to track deleted instructions
    Ordered,

    /// Multiple instructions can be executed as a single transaction
    /// Note: Transactions are not supported in the current version
    /// The implementation requires another account type to group instructions within a transaction
    UseTransaction,
}
