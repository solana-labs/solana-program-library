use std::{convert::TryInto, mem::size_of};

use solana_program::{
    instruction::{AccountMeta, Instruction},
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvar,
};

use crate::{
    error::TimelockError,
    state::{
        custom_single_signer_timelock_transaction::INSTRUCTION_LIMIT,
        enums::ConsensusAlgorithm,
        enums::ExecutionType,
        enums::TimelockType,
        timelock_config::TimelockConfig,
        timelock_state::{DESC_SIZE, NAME_SIZE},
    },
};

/// Used for telling caller what type of format you want back
#[derive(Clone, PartialEq)]
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
#[derive(Clone)]
pub enum TimelockInstruction {
    /// Initializes a new Timelock Program.
    ///
    ///   0. `[writable]` Timelock program account pub key.
    ///   1. `[]` Token program id
    ///   2. `[]` Rent sysvar
    InitTimelockProgram,

    /// Initializes a new empty Timelocked set of Instructions that will be executed at various slots in the future in draft mode.
    /// Grants Admin token to caller.
    ///
    ///   0. `[writable]` Uninitialized Timelock set account .
    ///   1. `[writable]` Initialized Signatory Mint account
    ///   2. `[writable]` Initialized Admin Mint account
    ///   3. `[writable]` Initialized Voting Mint account
    ///   4. `[writable]` Initialized Signatory Validation account
    ///   5. `[writable]` Initialized Admin Validation account
    ///   6. `[writable]` Initialized Voting Validation account
    ///   7. `[writable]` Initialized Destination account for first admin token
    ///   8. `[writable]` Initialized Destination account for first signatory token
    ///   9. `[]` Timelock program mint authority
    ///   10. `[]` Timelock Program
    ///   11. '[]` Token program id
    ///   12. `[]` Rent sysvar
    InitTimelockSet {
        /// Determine what type of timelock config you want
        config: TimelockConfig,
        /// Link to gist explaining proposal
        desc_link: [u8; DESC_SIZE],
        /// name of proposal
        name: [u8; NAME_SIZE],
    },

    /// [Requires Admin token]
    /// Adds a signatory to the Timelock which means that this timelock can't leave Draft state until yet another signatory burns
    /// their signatory token indicating they are satisfied with the instruction queue. They'll receive an signatory token
    /// as a result of this call that they can burn later.
    ///
    ///   0. `[writable]` Initialized new signatory account.
    ///   1. `[writable]` Initialized Signatory mint account.
    ///   2. `[writable]` Admin account.
    ///   3. `[writable]` Admin validation account.
    ///   4. `[]` Timelock set account.
    ///   5. `[]` Transfer authority
    ///   6. `[]` Timelock program mint authority
    ///   7. `[]` Timelock program account.
    ///   8. '[]` Token program id.
    AddSigner,

    /// [Requires Admin token]
    /// Removes a signer from the set.
    ///
    ///   0. `[writable]` Signatory account to remove token from.
    ///   1. `[writable]` Signatory mint account.
    ///   2. `[writable]` Admin account.
    ///   3. `[writable]` Admin validation account.
    ///   4. `[]` Timelock set account.
    ///   5. `[]` Transfer authority
    ///   5. `[]` Timelock program mint authority
    ///   6. `[]` Timelock program account.
    ///   7. '[]` Token program id.
    RemoveSigner,

    /// [Requires Signatory token]
    /// Adds a Transaction to the Timelock Set. Max of 10 of any Transaction type. More than 10 will throw error.
    /// Creates a PDA using your authority to be used to later execute the instruction.
    /// This transaction needs to contain authority to execute the program.
    ///
    ///   0. `[writable]` Uninitialized Timelock Transaction account.
    ///   1. `[writable]` Timelock set account.
    ///   2. `[writable]` Signatory account
    ///   3. `[writable]` Signatory validation account.
    ///   4. `[]` Transfer authority
    ///   5. `[]` Timelock mint authority
    ///   6. `[]` Timelock program account.
    ///   7. `[]` Token program account.
    AddCustomSingleSignerTransaction {
        /// Slot during which this will run
        slot: u64,
        /// Instruction
        instruction: [u8; INSTRUCTION_LIMIT],
        /// Position in transaction array
        position: u8,
        /// Point in instruction array where 0 padding begins - inclusive, index should be where actual instruction ends, not where 0s begin
        instruction_end_index: u16,
    },

    /// [Requires Signatory token]
    /// Remove Transaction from the Timelock Set.
    ///
    ///   0. `[writable]` Timelock set account.
    ///   1. `[writable]` Timelock Transaction account.
    ///   2. `[writable]` Signatory account
    ///   3. `[writable]` Signatory validation account.
    ///   4. `[]` Transfer Authority.
    ///   5. `[]` Timelock mint authority
    ///   6. `[]` Timelock program account pub key.
    ///   7. `[]` Token program account.
    RemoveTransaction,

    /// [Requires Signatory token]
    /// Update Transaction slot in the Timelock Set. Useful during reset periods.
    ///
    ///   0. `[writable]` Timelock set account.
    ///   1. `[writable]` Timelock Transaction account.
    ///   2. `[writable]` Signatory account
    ///   3. `[writable]` Signatory validation account.
    ///   4. `[]` Transfer authority.
    ///   5. `[]` Timelock mint authority
    ///   6. `[]` Timelock program account pub key.
    ///   7. `[]` Token program account.
    UpdateTransactionSlot {
        /// On what slot this transaction slot will now run
        slot: u64,
    },

    /// [Requires Admin token]
    /// Delete Timelock set entirely.
    ///
    ///   0. `[writable]` Timelock set account pub key.
    ///   1. `[writable]` Admin account
    ///   2. `[writable]` Admin validation account.
    ///   3. `[]` Transfer authority.
    ///   5. `[]` Timelock mint authority
    ///   6. `[]` Timelock program account pub key.
    ///   7. `[]` Token program account.
    DeleteTimelockSet,

    /// [Requires Signatory token]
    /// Burns signatory token, indicating you approve of moving this Timelock set from Draft state to Voting state.
    /// The last Signatory token to be burned moves the state to Voting.
    ///
    ///   0. `[writable]` Timelock set account pub key.
    ///   1. `[writable]` Signatory account
    ///   2. `[writable]` Signatory mint account.
    ///   3. `[]` Transfer authority
    ///   4. `[]` Timelock mint authority
    ///   5. `[]` Timelock program account pub key.
    ///   6. `[]` Token program account.
    Sign,

    /// [Requires Voting tokens]
    /// Burns voting tokens, indicating you approve of running this set of transactions. If you tip the consensus,
    /// then the transactions begin to be run at their time slots.
    ///
    ///   0. `[writable]` Timelock set account.
    ///   1. `[writable]` Voting account.
    ///   2. `[writable]` Voting mint account.
    ///   3. `[]` Transfer authority
    ///   4. `[]` Timelock program mint authority
    ///   5. `[]` Timelock program account pub key.
    ///   6. `[]` Token program account.
    Vote {
        /// How many voting tokens to burn
        voting_token_amount: u64,
    },

    /// [Requires Signatory token]
    /// Mints voting tokens for a destination account to be used during the voting process.
    ///
    ///   0. `[writable]` Timelock set account.
    ///   1. `[writable]` Initialized Voting account.
    ///   2. `[writable]` Voting mint account.
    ///   3. `[writable]` Signatory account
    ///   4. `[writable]` Signatory validation account.
    ///   5. `[]` Transfer authority
    ///   6. `[]` Timelock program mint authority
    ///   7. `[]` Timelock program account pub key.
    ///   8. `[]` Token program account.
    MintVotingTokens {
        /// How many voting tokens to mint
        voting_token_amount: u64,
    },

    /// Only used for testing. Requires no accounts of any kind.
    Ping,

    /// Executes a command in the timelock set.
    ///
    ///   0. `[writable]` Transaction account you wish to execute.
    ///   1. `[]` Timelock set account.
    ///   2. `[]` Program being invoked account
    ///   3. `[]` Timelock program authority
    ///   4. `[]` Timelock program account pub key.
    ///   5. `[]` Clock sysvar.
    ///   6+ Any extra accounts that are part of the instruction, in order
    Execute {
        /// Number of extra accounts
        number_of_extra_accounts: u8,
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
            1 => {
                let (consensus_algorithm, rest) = Self::unpack_u8(rest)?;
                let (execution_type, rest) = Self::unpack_u8(rest)?;
                let (timelock_type, rest) = Self::unpack_u8(rest)?;

                let (input_desc_link, input_name) = rest.split_at(DESC_SIZE);
                let mut desc_link: [u8; DESC_SIZE] = [0; DESC_SIZE];
                let mut name: [u8; NAME_SIZE] = [0; NAME_SIZE];
                for n in 0..(DESC_SIZE - 1) {
                    desc_link[n] = input_desc_link[n];
                }

                for n in 0..(NAME_SIZE - 1) {
                    name[n] = input_name[n];
                }
                Self::InitTimelockSet {
                    config: TimelockConfig {
                        consensus_algorithm: match consensus_algorithm {
                            0 => ConsensusAlgorithm::Majority,
                            1 => ConsensusAlgorithm::SuperMajority,
                            2 => ConsensusAlgorithm::FullConsensus,
                            _ => ConsensusAlgorithm::Majority,
                        },
                        execution_type: match execution_type {
                            0 => ExecutionType::AllOrNothing,
                            1 => ExecutionType::AnyAboveVoteFinishSlot,
                            _ => ExecutionType::AllOrNothing,
                        },
                        timelock_type: match timelock_type {
                            0 => TimelockType::CustomSingleSignerV1,
                            _ => TimelockType::CustomSingleSignerV1,
                        },
                    },
                    desc_link,
                    name,
                }
            }
            2 => Self::AddSigner,
            3 => Self::RemoveSigner,
            4 => {
                let (slot, rest) = Self::unpack_u64(rest)?;
                let (instruction, rest) = Self::unpack_instructions(rest)?;
                let (position, rest) = Self::unpack_u8(rest)?;
                let (instruction_end_index, _) = Self::unpack_u16(rest)?;
                Self::AddCustomSingleSignerTransaction {
                    slot,
                    instruction,
                    position,
                    instruction_end_index,
                }
            }
            5 => Self::RemoveTransaction,
            6 => {
                let (slot, _) = Self::unpack_u64(rest)?;
                Self::UpdateTransactionSlot { slot }
            }
            7 => Self::DeleteTimelockSet,
            8 => Self::Sign,
            9 => {
                let (voting_token_amount, _) = Self::unpack_u64(rest)?;
                Self::Vote {
                    voting_token_amount,
                }
            }
            10 => {
                let (voting_token_amount, _) = Self::unpack_u64(rest)?;
                Self::MintVotingTokens {
                    voting_token_amount,
                }
            }
            11 => Self::Ping,
            12 => {
                let (number_of_extra_accounts, _) = Self::unpack_u8(rest)?;
                Self::Execute {
                    number_of_extra_accounts,
                }
            }
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

    fn unpack_u16(input: &[u8]) -> Result<(u16, &[u8]), ProgramError> {
        if input.len() >= 2 {
            let (amount, rest) = input.split_at(2);
            let amount = amount
                .get(..2)
                .and_then(|slice| slice.try_into().ok())
                .map(u16::from_le_bytes)
                .ok_or(TimelockError::InstructionUnpackError)?;
            Ok((amount, rest))
        } else {
            Err(TimelockError::InstructionUnpackError.into())
        }
    }

    fn unpack_u32(input: &[u8]) -> Result<(u32, &[u8]), ProgramError> {
        if input.len() >= 4 {
            let (amount, rest) = input.split_at(4);
            let amount = amount
                .get(..4)
                .and_then(|slice| slice.try_into().ok())
                .map(u32::from_le_bytes)
                .ok_or(TimelockError::InstructionUnpackError)?;
            Ok((amount, rest))
        } else {
            Err(TimelockError::InstructionUnpackError.into())
        }
    }

    fn unpack_instructions(input: &[u8]) -> Result<([u8; INSTRUCTION_LIMIT], &[u8]), ProgramError> {
        if !input.is_empty() {
            if input.len() < INSTRUCTION_LIMIT {
                return Err(TimelockError::InstructionUnpackError.into());
            }

            let (input_instruction, rest) = input.split_at(INSTRUCTION_LIMIT);
            let mut instruction: [u8; INSTRUCTION_LIMIT] = [0; INSTRUCTION_LIMIT];
            for n in 0..(INSTRUCTION_LIMIT - 1) {
                instruction[n] = input_instruction[n];
            }
            Ok((instruction, rest))
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

        match self {
            Self::InitTimelockProgram => {
                buf.push(0);
            }
            Self::InitTimelockSet {
                config,
                desc_link,
                name,
            } => {
                buf.push(1);
                match config.consensus_algorithm {
                    ConsensusAlgorithm::Majority => buf.push(0),
                    ConsensusAlgorithm::SuperMajority => buf.push(1),
                    ConsensusAlgorithm::FullConsensus => buf.push(2),
                }
                match config.execution_type {
                    ExecutionType::AllOrNothing => buf.push(0),
                    ExecutionType::AnyAboveVoteFinishSlot => buf.push(1),
                }
                match config.timelock_type {
                    TimelockType::CustomSingleSignerV1 => buf.push(0),
                }
                buf.extend_from_slice(desc_link);
                buf.extend_from_slice(name);
            }
            Self::AddSigner => buf.push(2),
            Self::RemoveSigner => buf.push(3),
            Self::AddCustomSingleSignerTransaction {
                slot,
                instruction,
                position,
                instruction_end_index,
            } => {
                buf.push(4);
                buf.extend_from_slice(&slot.to_le_bytes());
                buf.extend_from_slice(instruction);
                buf.extend_from_slice(&position.to_le_bytes());
                buf.extend_from_slice(&instruction_end_index.to_le_bytes());
            }
            Self::RemoveTransaction {} => buf.push(5),
            Self::UpdateTransactionSlot { slot } => {
                buf.push(6);
                buf.extend_from_slice(&slot.to_le_bytes());
            }
            Self::DeleteTimelockSet => buf.push(7),
            Self::Sign => buf.push(8),
            Self::Vote {
                voting_token_amount,
            } => {
                buf.push(9);
                buf.extend_from_slice(&voting_token_amount.to_le_bytes());
            }
            Self::MintVotingTokens {
                voting_token_amount,
            } => {
                buf.push(10);
                buf.extend_from_slice(&voting_token_amount.to_le_bytes());
            }
            Self::Ping => buf.push(11),
            Self::Execute {
                number_of_extra_accounts,
            } => {
                buf.push(12);
                buf.extend_from_slice(&number_of_extra_accounts.to_le_bytes());
            }
        }
        buf
    }
}

/// Creates an 'InitTimelockProgram' instruction.
pub fn init_timelock_program(
    program_id: Pubkey,
    timelock_pubkey: Pubkey,
    token_program: Pubkey,
) -> Instruction {
    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(timelock_pubkey, true),
            AccountMeta::new_readonly(token_program, false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data: TimelockInstruction::InitTimelockProgram.pack(),
    }
}
