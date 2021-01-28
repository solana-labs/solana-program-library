const TRANSACTION_SLOTS: usize = 10;
pub(crate) const TIMELOCK_VERSION: u8 = 1;
const UNINITIALIZED_VERSION: u8 = 0;
pub const INSTRUCTION_LIMIT: usize = 2_000_000;

pub mod enums;
pub mod timelock_program;
use self::enums::{ConsensusAlgorithm, ExecutionType, TimelockStateStatus, TimelockType};
use solana_program::pubkey::Pubkey;

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

#[derive(Clone, Debug, PartialEq)]
pub struct CustomSingleSignerV1TimelockTransaction {
    /// Slot at which this will execute
    slot: u64,

    instruction: [u8; INSTRUCTION_LIMIT],

    /// authority key (pda) used to run the program
    authority_key: Pubkey,
}
