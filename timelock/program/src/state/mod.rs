use crate::{
    error::LendingError,
    math::{Decimal, Rate, SCALE},
};
use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use solana_program::{
    clock::{Slot, DEFAULT_TICKS_PER_SECOND, DEFAULT_TICKS_PER_SLOT, SECONDS_PER_DAY},
    entrypoint::ProgramResult,
    program_error::ProgramError,
    program_option::COption,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
    sysvar::clock::Clock,
};

const TRANSACTION_SLOTS: u8 = 10;
const TIMELOCK_VERSION: u8 = 1;
pub const INSTRUCTION_LIMIT: u64 = 2_000_000;

pub enum ConsensusAlgorithm {
    /// Run if 51% of tokens are burned in favor of the timelock set
    Majority,
    /// Run if 66% of tokens are burned in favor
    SuperMajority,
    /// Run only if 100% of tokens are burned in favor
    FullConsensus,
}

pub enum ExecutionType {
    /// Only run the timelock set if all of the transactions have slot times above the slot that the vote finished at
    AllOrNothing,
    /// Run the remaining set transactions whose slots are above the slot the vote finished at
    AnyAboveVoteFinishSlot,
}

pub enum TimelockStateStatus {
    Draft,
    Voting,
    VoteComplete,
}

pub enum TimelockType {
    /// Only supported type for now - call the Upgrade program
    Upgrade,
}

/// Global app state
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TimelockProgram {
    /// Version of app
    pub version: u8,
    /// program id
    pub program_id: Pubkey,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TimelockSet {
    /// Version of the struct
    pub version: u8,

    /// Mint that creates signatory tokens of this instruction
    /// If there are outstanding signatory tokens, then cannot leave draft state. Signatories must burn tokens (ie agree
    /// to move instruction to voting state) and bring mint to net 0 tokens outstanding. Each signatory gets 1 (serves as flag)
    pub signatory_mint: Pubkey,

    /// Admin ownership mint. One token is minted, can be used to grant admin status to a new person.
    pub admin_mint: Pubkey,

    /// Mint that creates voting tokens of this instruction
    pub voting_mint: Pubkey,

    /// Reserve state
    pub state: TimelockState,

    /// configuration values
    pub config: TimelockConfig,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TimelockState {
    /// Current state of the invoked instruction account
    pub status: TimelockStateStatus,

    /// Total voting tokens minted, for use comparing to supply remaining during consensus
    pub total_voting_tokens_minted: u64,

    /// Array of pubkeys pointing at TimelockTransactions, up to 10
    pub timelock_transactions: [Pubkey; TRANSACTION_SLOTS],
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TimelockConfig {
    consensus_algorithm: ConsensusAlgorithm,
    execution_type: ExecutionType,
    timelock_type: TimelockType,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct UpgradeTimelockTransaction {
    /// Slot at which this will execute
    slot: u64,

    /// Executable location where new program resides
    new_program_temp_location: Pubkey,

    /// Program being upgraded
    program_id_to_upgrade: Pubkey,

    /// Executor program id
    executor_program_id: Pubkey,

    /// authority key (pda) used to run the program
    authority_key: Pubkey,
}
