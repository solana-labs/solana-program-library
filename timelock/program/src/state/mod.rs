const TRANSACTION_SLOTS: usize = 10;

const UNINITIALIZED_VERSION: u8 = 0;
/// Max instruction limit for generics
pub const INSTRUCTION_LIMIT: usize = 2_000_000;

/// Enums
pub mod enums;
/// Timelock config
pub mod timelock_config;
/// Timelock program
pub mod timelock_program;
/// Timelock set
pub mod timelock_set;
/// Timelock state
pub mod timelock_state;
use solana_program::pubkey::Pubkey;

/// First iteration of generic instruction
#[derive(Clone, Debug, PartialEq)]
pub struct CustomSingleSignerV1TimelockTransaction {
    /// Slot at which this will execute
    slot: u64,

    /// Instruction set
    instruction: [u8; INSTRUCTION_LIMIT],

    /// authority key (pda) used to run the program
    authority_key: Pubkey,
}
