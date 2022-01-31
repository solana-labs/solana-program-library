//! State enumerations

use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};

/// Defines all Governance accounts types
#[repr(C)]
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub enum GovernanceAccountType {
    /// Default uninitialized account state
    Uninitialized,

    /// Top level aggregation for governances with Community Token (and optional Council Token)
    RealmV1,

    /// Token Owner Record for given governing token owner within a Realm
    TokenOwnerRecordV1,

    /// Governance account
    GovernanceV1,

    /// Program Governance account
    ProgramGovernanceV1,

    /// Proposal account for Governance account. A single Governance account can have multiple Proposal accounts
    ProposalV1,

    /// Proposal Signatory account
    SignatoryRecordV1,

    /// Vote record account for a given Proposal.  Proposal can have 0..n voting records
    VoteRecordV1,

    /// ProposalInstruction account which holds an instruction to execute for Proposal
    ProposalInstructionV1,

    /// Mint Governance account
    MintGovernanceV1,

    /// Token Governance account
    TokenGovernanceV1,

    /// Realm config account (introduced in V2)
    RealmConfig,

    /// Vote record account for a given Proposal.  Proposal can have 0..n voting records
    /// V2 adds support for multi option votes
    VoteRecordV2,

    /// ProposalTransaction account which holds instructions to execute for Proposal within a single Transaction
    /// V2 replaces ProposalInstruction and adds index for proposal option and multiple instructions
    ProposalTransactionV2,

    /// Proposal account for Governance account. A single Governance account can have multiple Proposal accounts
    /// V2 adds support for multiple vote options
    ProposalV2,

    /// Program metadata account (introduced in V2)
    /// It stores information about the particular SPL-Governance program instance
    ProgramMetadata,

    /// Top level aggregation for governances with Community Token (and optional Council Token)
    /// V2 adds the following fields:
    /// 1) use_community_voter_weight_addin and use_max_community_voter_weight_addin to RealmConfig
    /// 2) voting_proposal_count
    /// 3) extra reserved space reserved_v2
    RealmV2,

    /// Token Owner Record for given governing token owner within a Realm
    /// V2 adds extra reserved space reserved_v2
    TokenOwnerRecordV2,

    /// Governance account
    /// V2 adds extra reserved space reserved_v2
    GovernanceV2,

    /// Program Governance account
    /// V2 adds extra reserved space reserved_v2
    ProgramGovernanceV2,

    /// Mint Governance account
    /// V2 adds extra reserved space reserved_v2
    MintGovernanceV2,

    /// Token Governance account
    /// V2 adds extra reserved space reserved_v2
    TokenGovernanceV2,

    /// Proposal Signatory account
    /// V2 adds extra reserved space reserved_v2
    SignatoryRecordV2,
}

impl Default for GovernanceAccountType {
    fn default() -> Self {
        GovernanceAccountType::Uninitialized
    }
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

    /// Voting on Proposal succeeded and now instructions are being executed
    /// Proposal enter this state when first instruction is executed and leaves when the last instruction is executed
    Executing,

    /// Completed
    Completed,

    /// Cancelled
    Cancelled,

    /// Defeated
    Defeated,

    /// Same as Executing but indicates some instructions failed to execute
    /// Proposal can't be transitioned from ExecutingWithErrors to Completed state
    ExecutingWithErrors,
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

/// The type of vote tipping to use on a Proposal.
///
/// Vote tipping means that under some conditions voting will complete early.
#[repr(C)]
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub enum VoteTipping {
    /// Tip when there is no way for another option to win and the vote threshold
    /// has been reached. This ignores voters withdrawing their votes.
    ///
    /// Currently only supported for the "yes" option in single choice votes.
    Strict,

    /// Tip when an option reaches the vote threshold and has more vote weight
    /// than any other options.
    ///
    /// Currently only supported for the "yes" option in single choice votes.
    Early,

    /// Never tip the vote early.
    Disabled,
}

/// The status of instruction execution
#[repr(C)]
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub enum TransactionExecutionStatus {
    /// Transaction was not executed yet
    None,

    /// Transaction was executed successfully
    Success,

    /// Transaction execution failed
    Error,
}

/// Transaction execution flags defining how instructions are executed for a Proposal
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

/// The source of max vote weight used for voting
/// Values below 100% mint supply can be used when the governing token is fully minted but not distributed yet
#[repr(C)]
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub enum MintMaxVoteWeightSource {
    /// Fraction (10^10 precision) of the governing mint supply is used as max vote weight
    /// The default is 100% (10^10) to use all available mint supply for voting
    SupplyFraction(u64),

    /// Absolute value, irrelevant of the actual mint supply, is used as max vote weight
    /// Note: this option is not implemented in the current version
    Absolute(u64),
}

impl MintMaxVoteWeightSource {
    /// Base for mint supply fraction calculation
    pub const SUPPLY_FRACTION_BASE: u64 = 10_000_000_000;

    /// 100% of mint supply
    pub const FULL_SUPPLY_FRACTION: MintMaxVoteWeightSource =
        MintMaxVoteWeightSource::SupplyFraction(MintMaxVoteWeightSource::SUPPLY_FRACTION_BASE);
}
