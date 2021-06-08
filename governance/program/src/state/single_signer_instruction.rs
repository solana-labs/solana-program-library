//! SingleSignerInstruction Account

use crate::state::enums::GovernanceAccountType;
use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};

/// Account for an instruction to be executed for Proposal
#[repr(C)]
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub struct SingleSignerInstruction {
    /// Governance Account type
    pub account_type: GovernanceAccountType,

    /// Minimum waiting time in slots for the  instruction to be executed once proposal is voted on
    pub hold_up_time: u64,

    /// Instruction to execute
    /// The instruction will be signed by Governance PDA the Proposal belongs to
    // For example for ProgramGovernance the instruction to upgrade program will be signed by ProgramGovernance PDA
    pub instruction: InstructionData,

    /// Executed flag
    pub executed: bool,
}

/// Temp. placeholder until I get Borsh serialization for Instruction working
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
#[repr(C)]
pub struct InstructionData {}
