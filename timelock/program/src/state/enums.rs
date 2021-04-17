/// Defines all persistent struct types and their versions
#[derive(Clone, Debug, PartialEq)]
pub enum GovernanceAccountType {
    /// 0 - Default uninitialized account state
    Uninitialized,

    /// 1 - Governance configuration account
    Governance,

    /// 2 - Proposal account for Governance account. A single Governance account can have multiple Proposal accounts
    Proposal,

    /// 3 - Proposal voting state account. Every Proposal account has exactly one ProposalState account
    ProposalState,

    /// 4 - Vote record account for a given Proposal.  Proposal can have 0..n voting records
    VoteRecord,

    /// 5 Custom Single Signer Transaction account which holds instructions to execute for Proposal
    CustomSingleSignerTransaction,
}

impl Default for GovernanceAccountType {
    fn default() -> Self {
        GovernanceAccountType::Uninitialized
    }
}

/// What type of execution a timelock is
#[derive(Clone, Debug, PartialEq)]
pub enum ExecutionType {
    /// Each transaction is independent of the other
    Independent,
}

impl Default for ExecutionType {
    fn default() -> Self {
        ExecutionType::Independent
    }
}

/// What state a timelock set is in
#[derive(Clone, Debug, PartialEq)]
pub enum ProposalStateStatus {
    /// Draft
    Draft,
    /// Taking votes
    Voting,

    /// Votes complete, in execution phase
    Executing,

    /// Completed, can be rebooted
    Completed,

    /// Deleted
    Deleted,

    /// Defeated
    Defeated,
}

impl Default for ProposalStateStatus {
    fn default() -> Self {
        ProposalStateStatus::Draft
    }
}

/// What type a timelock is
#[derive(Clone, Debug, PartialEq)]
pub enum TimelockType {
    /// Meaning voters deposit their tokens from outside mint and "govern" the proposal
    Governance,
}

impl Default for TimelockType {
    fn default() -> Self {
        TimelockType::Governance
    }
}

/// Rules for voters entering the timelock
#[derive(Clone, Debug, PartialEq)]
pub enum VotingEntryRule {
    /// Meaning voters can enter at any point in time
    Anytime,
}

impl Default for VotingEntryRule {
    fn default() -> Self {
        VotingEntryRule::Anytime
    }
}
