use std::{convert::TryInto, mem::size_of};

use solana_program::program_error::ProgramError;

use crate::{
    error::TimelockError,
    state::{TimelockConfig, INSTRUCTION_LIMIT},
};

/// Used for telling caller what type of format you want back
#[derive(Clone, Debug, PartialEq)]
pub enum Format {
    /// JSON format
    JSON,
    /// MsgPack format
    MsgPack,
}
impl Default for Format {
    fn default() -> Self {
        Format::JSON
    }
}

/// Instructions supported by the Timelock program.
#[derive(Clone, Debug, PartialEq)]
pub enum TimelockInstruction {
    /// Initializes a new Timelock Program.
    ///
    ///   0. `[writable]` Timelock program account pub key.
    ///   1. `[]` Rent sysvar
    InitTimelockProgram,

    /// Initializes a new empty Timelocked set of Instructions that will be executed at various slots in the future in draft mode.
    /// Grants Admin token to caller.
    ///
    ///   0. `[writable]` Timelock set account pub key.
    ///   1. `[]` Timelock program account pub key.
    ///   2. `[]` Rent sysvar
    ///   3. '[]` Token program id
    InitTimelockSet {
        /// Determine what type of timelock config you want
        config: TimelockConfig,
    },

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
    /// Adds an Upgrade type Transaction to the Timelock Set. Max of 10 of any Transaction type. More than 10 will throw error.
    /// Creates a PDA using your authority to be used to later execute the upgrade program.
    /// This transaction needs to contain authority to execute the executor program and to write to the program account you are
    /// upgrading as the executor.
    ///
    ///   0. `[writable]` Timelock set account pub key.
    ///   1. `[writable]` Pubkey for use creating new Timelock Transaction account.
    ///   2. `[]` Timelock program account pub key.
    ///   3. `[]` Executor program account pub key.
    AddCustomSingleSignerV1Transaction {
        /// Slot during which this will run
        slot: u64,
        /// Instruction
        instruction: [u64; INSTRUCTION_LIMIT],
    },

    /// [Requires Signatory token]
    /// Remove Transaction from the Timelock Set.
    ///
    ///   0. `[writable]` Timelock set account pub key.
    ///   1. `[writable]` Timelock Transaction pub key.
    ///   2. `[]` Timelock program account pub key.
    RemoveTransaction {},

    /// [Requires Signatory token]
    /// Update Transaction slot in the Timelock Set. Useful during reset periods.
    ///
    ///   0. `[writable]` Timelock Transaction pub key.
    ///   1. `[]` Timelock set account pub key.
    ///   1. `[]` Timelock program account pub key.
    UpdateTransactionSlot {
        /// On what slot this transaction slot will now run
        slot: u64,
    },

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
    ///   0. `[writable]` Timelock set account pub key.
    ///   1. `[]` Timelock program account pub key.
    Sign {},

    /// [Requires Voting tokens]
    /// Burns voting tokens, indicating you approve of running this set of transactions. If you tip the consensus,
    /// then the transactions begin to be run at their time slots.
    ///
    ///   0. `[]` Timelock set account pub key.
    ///   1. `[]` Timelock program account pub key.
    Vote {
        /// How many voting tokens to burn
        voting_token_amount: u64,
    },

    /// [Requires Signatory token]
    /// Mints voting tokens for a destination account to be used during the voting process.
    ///
    ///   0. `[writable]` Timelock set account pub key.
    ///   1. `[]` Timelock program account pub key.
    ///   2. `[]` Destination account pub key.
    MintVotingTokens {
        /// How many voting tokens to mint
        voting_token_amount: u64,
    },
}

impl TimelockInstruction {
    /// Unpacks a byte buffer into a [TimelockInstruction](enum.TimelockInstruction.html).
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        let (&tag, rest) = input
            .split_first()
            .ok_or(TimelockError::InstructionUnpackError)?;
        Ok(match tag {
            0 => Self::InitTimelockProgram,
            _ => return Err(TimelockError::InstructionUnpackError.into()),
        })
    }

    fn unpack_u64(input: &[u8]) -> Result<(u64, &[u8]), ProgramError> {
        if input.len() >= 8 {
            let (amount, rest) = input.split_at(8);
            let amount = amount
                .get(..8)
                .and_then(|slice| slice.try_into().ok())
                .map(u64::from_le_bytes)
                .ok_or(TimelockError::InstructionUnpackError)?;
            Ok((amount, rest))
        } else {
            Err(TimelockError::InstructionUnpackError.into())
        }
    }

    fn unpack_u8(input: &[u8]) -> Result<(u8, &[u8]), ProgramError> {
        if !input.is_empty() {
            let (amount, rest) = input.split_at(1);
            let amount = amount
                .get(..1)
                .and_then(|slice| slice.try_into().ok())
                .map(u8::from_le_bytes)
                .ok_or(TimelockError::InstructionUnpackError)?;
            Ok((amount, rest))
        } else {
            Err(TimelockError::InstructionUnpackError.into())
        }
    }

    /// Packs a [TimelockInstruction](enum.TimelockInstruction.html) into a byte buffer.
    pub fn pack(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(size_of::<Self>());
        match *self {
            Self::InitTimelockProgram => {
                buf.push(0);
            }
            Self::InitTimelockProgram => {
                buf.push(1);
            }
            Self::InitTimelockSet {
                config: TimelockConfig { .. },
            } => {
                buf.push(2);
            }
            Self::AddSigner => {}
            Self::RemoveSigner => {}
            Self::AddCustomSingleSignerV1Transaction { slot, instruction } => {}
            Self::RemoveTransaction {} => {}
            Self::UpdateTransactionSlot { slot } => {}
            Self::DeleteTimelockSet {} => {}
            Self::Sign {} => {}
            Self::Vote {
                voting_token_amount,
            } => {}
            Self::MintVotingTokens {
                voting_token_amount,
            } => {}
        }
        buf
    }
}
