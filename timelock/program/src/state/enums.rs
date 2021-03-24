/// What kind of consensus algo a timelock uses
#[derive(Clone, Debug, PartialEq)]
pub enum ConsensusAlgorithm {
    /// Run if 51% of tokens are burned in favor of the timelock set
    Majority,
    /// Run if 66% of tokens are burned in favor
    SuperMajority,
    /// Run only if 100% of tokens are burned in favor
    FullConsensus,
}

impl Default for ConsensusAlgorithm {
    fn default() -> Self {
        ConsensusAlgorithm::Majority
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
pub enum TimelockStateStatus {
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

impl Default for TimelockStateStatus {
    fn default() -> Self {
        TimelockStateStatus::Draft
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
