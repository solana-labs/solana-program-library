use super::enums;
use enums::{ConsensusAlgorithm, ExecutionType, TimelockType};
/// Timelock Config
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TimelockConfig {
    /// Consensus Algorithm
    pub consensus_algorithm: ConsensusAlgorithm,
    /// Execution type
    pub execution_type: ExecutionType,
    /// Timelock Type
    pub timelock_type: TimelockType,
}
