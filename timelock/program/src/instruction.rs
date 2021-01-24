use crate::{
    error::LendingError,
    state::{ReserveConfig, ReserveFees},
};
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::{FromPrimitive, ToPrimitive};
use solana_program::{
    instruction::{AccountMeta, Instruction},
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvar,
};
use std::{convert::TryInto, mem::size_of};

pub enum Format {
    JSON,
    MsgPack,
}
/// Instructions supported by the Timelock program.
#[derive(Clone, Debug, PartialEq)]
pub enum TimelockInstruction {
    /// Initializes a new Timelock Program.
    ///
    ///   0. `[writable]` Timelock program account pub key.
    ///   2. `[]` Rent sysvar
    ///   3. '[]` Token program id
    InitTimelockProgram,

    /// Initializes a new empty Timelocked set of Instructions that will be executed at various slots in the future in draft mode.
    /// Grants Admin token to caller.
    ///
    ///   0. `[writable]` Timelock set account pub key.
    ///   1. `[]` Timelock program account pub key.
    ///   2. `[]` Rent sysvar
    ///   3. '[]` Token program id
    InitTimelockSet { config: TimelockConfig },

    /// [Requires Admin token]
    /// Adds a signatory to the Timelock which means that this timelock can't leave Draft state until yet another signatory burns
    /// their signatory token indicating they are satisfied with the instruction queue. They'll receive an signatory token
    /// as a result of this call that they can burn later.
    ///
    ///   0. `[]` Timelock set account pub key.
    ///   1. `[]` New signatory account pub key.
    ///   2. `[]` Timelock program account pub key.
    ///   3. '[]` Token program id
    AddSigner,

    /// [Requires Admin token]
    /// Removes a signer from the set.
    ///
    ///   0. `[]` Timelock set account pub key.
    ///   1. `[]` Signer account pub key (cannot be yourself)
    ///   2. `[]` Timelock program account pub key.
    ///   3. '[]` Token program id
    RemoveSigner,

    /// [Requires Signatory token]
    /// Adds Transaction to the Timelock Set. Max of 10. More than 10 will throw error.
    ///
    ///   0. `[writable]` Timelock set account pub key.
    ///   1. `[]` Timelock program account pub key.
    ///   2. `[]` executable pub key. Must have granted executable authority to Timelock program account pub key in advance.
    AddTransaction {
        /// Slot during which this executable will run.
        slot: u64,
    },

    /// [Requires Signatory token]
    /// Remove Transaction from the Timelock Set.
    ///
    ///   0. `[writable]` Timelock set account pub key.
    ///   1. `[]` Timelock program account pub key.
    ///   2. `[]` executable pub key.
    RemoveTransaction {},

    /// [Requires Admin token]
    /// Delete Timelock set entirely.
    ///
    ///   0. `[writable]` Timelock set account pub key.
    ///   1. `[]` Timelock program account pub key.
    DeleteTimelockSet {},

    /// [Requires Signatory token]
    /// Burns signatory token, indicating you approve of moving this Timelock set from Draft state to Voting state.
    /// The last Signatory token to be burned moves the state to Voting.
    ///
    ///   0. `[]` Timelock set account pub key.
    ///   1. `[]` Timelock program account pub key.
    Sign {},

    /// [Requires Voting tokens]
    /// Burns voting tokens, indicating you approve of running this set of transactions. If you tip the consensus,
    /// then the transactions begin to be run at their time slots.
    ///
    ///   0. `[]` Timelock set account pub key.
    ///   1. `[]` Timelock program account pub key.
    Vote { voting_token_amount: u64 },

    /// [Requires Signatory token]
    /// Mints voting tokens for a destination account to be used during the voting process.
    ///
    ///   0. `[]` Timelock set account pub key.
    ///   1. `[]` Timelock program account pub key.
    ///   2. `[]` Destination account pub key.
    MintVotingTokens { voting_token_amount: u64 },

    /// Gets status of Timelock Set, returns it's entire state as JSON or MsgPack.
    ///
    ///   0. `[]` Timelock set account pub key.
    ///   1. `[]` Timelock program account pub key.
    Status { format: Format },
}
