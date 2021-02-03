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
    /// Only run the timelock set if all of the transactions have slot times above the slot that the vote finished at
    AllOrNothing,
    /// Run the remaining set transactions whose slots are above the slot the vote finished at
    AnyAboveVoteFinishSlot,
}

impl Default for ExecutionType {
    fn default() -> Self {
        ExecutionType::AllOrNothing
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
}

impl Default for TimelockStateStatus {
    fn default() -> Self {
        TimelockStateStatus::Draft
    }
}

/// What type a timelock is
#[derive(Clone, Debug, PartialEq)]
pub enum TimelockType {
    /// Only supported type for now - call the Upgrade program
    CustomSingleSignerV1,
}

impl Default for TimelockType {
    fn default() -> Self {
        TimelockType::CustomSingleSignerV1
    }
}
