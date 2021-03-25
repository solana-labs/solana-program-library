use std::{convert::TryInto, mem::size_of};

use solana_program::{
    instruction::{AccountMeta, Instruction},
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvar,
};

use crate::{
    error::TimelockError,
    state::{
        custom_single_signer_timelock_transaction::INSTRUCTION_LIMIT,
        timelock_config::CONFIG_NAME_LENGTH,
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
    ///   0. `[writable]` Uninitialized Timelock state account .
    ///   1. `[writable]` Uninitialized Timelock set account .
    ///   2. `[writable]` Initialized Timelock config account.
    ///   3. `[writable]` Initialized Signatory Mint account
    ///   4. `[writable]` Initialized Admin Mint account
    ///   5. `[writable]` Initialized Voting Mint account
    ///   6. `[writable]` Initialized Yes Voting Mint account
    ///   7. `[writable]` Initialized No Voting Mint account
    ///   8. `[writable]` Initialized Signatory Validation account
    ///   9. `[writable]` Initialized Admin Validation account
    ///   10. `[writable]` Initialized Voting Validation account
    ///   11. `[writable]` Initialized Destination account for first admin token
    ///   12. `[writable]` Initialized Destination account for first signatory token
    ///   13. `[writable]` Initialized Yes voting dump account
    ///   14. `[writable]` Initialized No voting dump account
    ///   15. `[writable]` Initialized source holding account
    ///   16. `[]` Source mint
    ///   17. `[]` Timelock minting authority
    ///   18. `[]` Timelock Program
    ///   19. '[]` Token program id
    ///   20. `[]` Rent sysvar
    InitTimelockSet {
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
    ///   5. `[writable]` Timelock state account.
    ///   6. `[]` Timelock set account.
    ///   7. `[]` Transfer authority
    ///   8. `[]` Timelock program mint authority
    ///   9. `[]` Timelock program account.
    ///   10. '[]` Token program id.
    AddSigner,

    /// [Requires Admin token]
    /// Removes a signer from the set.
    ///
    ///   0. `[writable]` Signatory account to remove token from.
    ///   1. `[writable]` Signatory mint account.
    ///   2. `[writable]` Admin account.
    ///   3. `[writable]` Admin validation account.
    ///   4. `[writable]` Timelock state account.
    ///   5. `[]` Timelock set account.
    ///   6. `[]` Transfer authority
    ///   7. `[]` Timelock program mint authority
    ///   8. `[]` Timelock program account.
    ///   9. '[]` Token program id.
    RemoveSigner,

    /// [Requires Signatory token]
    /// Adds a Transaction to the Timelock Set. Max of 10 of any Transaction type. More than 10 will throw error.
    /// Creates a PDA using your authority to be used to later execute the instruction.
    /// This transaction needs to contain authority to execute the program.
    ///
    ///   0. `[writable]` Uninitialized Timelock Transaction account.
    ///   1. `[writable]` Timelock state account.
    ///   2. `[writable]` Signatory account
    ///   3. `[writable]` Signatory validation account.
    ///   4. `[]` Timelock Set account.
    ///   5. `[]` Timelock Config account.
    ///   6. `[]` Transfer authority
    ///   7. `[]` Timelock mint authority
    ///   8. `[]` Timelock program account.
    ///   9. `[]` Token program account.
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
    ///   0. `[writable]` Timelock state account.
    ///   1. `[writable]` Timelock Transaction account.
    ///   2. `[writable]` Signatory account
    ///   3. `[writable]` Signatory validation account.
    ///   5. `[]` Timelock set.
    ///   6. `[]` Transfer Authority.
    ///   7. `[]` Timelock mint authority
    ///   8. `[]` Timelock program account pub key.
    ///   9. `[]` Token program account.
    RemoveTransaction,

    /// [Requires Signatory token]
    /// Update Transaction slot in the Timelock Set. Useful during reset periods.
    ///
    ///   1. `[writable]` Timelock Transaction account.
    ///   2. `[writable]` Signatory account
    ///   3. `[writable]` Signatory validation account.
    ///   4. `[]` Timelock state account.
    ///   5. `[]` Timelock set account.
    ///   6. `[]` Transfer authority.
    ///   7. `[]` Timelock mint authority
    ///   8. `[]` Timelock program account pub key.
    ///   9. `[]` Token program account.
    UpdateTransactionSlot {
        /// On what slot this transaction slot will now run
        slot: u64,
    },

    /// [Requires Admin token]
    /// Delete Timelock set entirely.
    ///
    ///   0. `[writable]` Timelock state account pub key.
    ///   1. `[writable]` Admin account
    ///   2. `[writable]` Admin validation account.
    ///   3. `[]` Timelock set account pub key.
    ///   4. `[]` Transfer authority.
    ///   5. `[]` Timelock mint authority
    ///   6. `[]` Timelock program account pub key.
    ///   7. `[]` Token program account.
    DeleteTimelockSet,

    /// [Requires Signatory token]
    /// Burns signatory token, indicating you approve of moving this Timelock set from Draft state to Voting state.
    /// The last Signatory token to be burned moves the state to Voting.
    ///
    ///   0. `[writable]` Timelock state account pub key.
    ///   1. `[writable]` Signatory account
    ///   2. `[writable]` Signatory mint account.
    ///   3. `[]` Timelock set account pub key.
    ///   4. `[]` Transfer authority
    ///   5. `[]` Timelock mint authority
    ///   6. `[]` Timelock program account pub key.
    ///   7. `[]` Token program account.
    ///   8. `[]` Clock sysvar.
    Sign,

    /// [Requires Voting tokens]
    /// Burns voting tokens, indicating you approve and/or disapprove of running this set of transactions. If you tip the consensus,
    /// then the transactions can begin to be run at their time slots when people click execute.
    ///
    ///   0. `[writable]` Timelock state account.
    ///   1. `[writable]` Your Voting account.
    ///   2. `[writable]` Your Yes-Voting account.
    ///   3. `[writable]` Your No-Voting account.
    ///   4. `[writable]` Voting mint account.
    ///   5. `[writable]` Yes Voting mint account.
    ///   6. `[writable]` No Voting mint account.
    ///   7. `[]` Source mint account
    ///   8. `[]` Timelock set account.
    ///   9. `[]` Timelock config account.
    ///   10. `[]` Transfer authority
    ///   11. `[]` Timelock program mint authority
    ///   12. `[]` Timelock program account pub key.
    ///   13. `[]` Token program account.
    ///   14. `[]` Clock sysvar.
    Vote {
        /// How many voting tokens to burn yes
        yes_voting_token_amount: u64,
        /// How many voting tokens to burn no
        no_voting_token_amount: u64,
    },

    /// Only used for testing. Requires no accounts of any kind.
    Ping,

    /// Executes a command in the timelock set.
    ///
    ///   0. `[writable]` Transaction account you wish to execute.
    ///   1. `[writable]` Timelock state account.
    ///   2. `[]` Program being invoked account
    ///   3. `[]` Timelock set account.
    ///   4. `[]` Timelock config
    ///   5. `[]` Timelock program account pub key.
    ///   6. `[]` Clock sysvar.
    ///   7+ Any extra accounts that are part of the instruction, in order
    Execute {
        /// Number of extra accounts
        number_of_extra_accounts: u8,
    },

    /// [Requires tokens of the Governance mint or Council mint depending on type of TimelockSet]
    /// Deposits voting tokens to be used during the voting process in a timelock.
    /// These tokens are removed from your account and can be returned by withdrawing
    /// them from the timelock (but then you will miss the vote.)
    ///
    ///   0. `[writable]` Initialized Voting account to hold your received voting tokens.
    ///   1. `[writable]` User token account to deposit tokens from.
    ///   2. `[writable]` Source holding account for timelock that will accept the tokens in escrow.
    ///   3. `[writable]` Voting mint account.
    ///   4. `[]` Timelock set account.
    ///   5. `[]` Transfer authority
    ///   6. `[]` Timelock program mint authority
    ///   7. `[]` Timelock program account pub key.
    ///   8. `[]` Token program account.
    DepositSourceTokens {
        /// How many voting tokens to deposit
        voting_token_amount: u64,
    },

    /// [Requires voting tokens]
    /// Withdraws voting tokens.
    ///
    ///   0. `[writable]` Initialized Voting account from which to remove your voting tokens.
    ///   1. `[writable]` Initialized Yes Voting account from which to remove your voting tokens.
    ///   2. `[writable]` Initialized No Voting account from which to remove your voting tokens.
    ///   3. `[writable]` User token account that you wish your actual tokens to be returned to.
    ///   4. `[writable]` Source holding account owned by the timelock that will has the actual tokens in escrow.
    ///   5. `[writable]` Initialized Yes Voting dump account owned by timelock set to which to send your voting tokens.
    ///   6. `[writable]` Initialized No Voting dump account owned by timelock set to which to send your voting tokens.
    ///   7. `[]` Voting mint account.
    ///   8. `[]` Timelock state account.
    ///   9. `[]` Timelock set account.
    ///   10. `[]` Transfer authority
    ///   11. `[]` Yes Transfer authority
    ///   12. `[]` No Transfer authority
    ///   13. `[]` Timelock program mint authority
    ///   14. `[]` Timelock program account pub key.
    ///   15. `[]` Token program account.
    WithdrawVotingTokens {
        /// How many voting tokens to withdrawal
        voting_token_amount: u64,
    },

    ///   0. `[writable]` Timelock config key. Needs to be set with pubkey set to PDA with seeds of the
    ///           program account key, governance mint key, council mint key, timelock program account key.
    ///   1. `[]` Program account that this config uses
    ///   2. `[]` Governance mint that this config uses
    ///   3. `[]` Council mint that this config uses [Optional] [Pass in 0s otherwise]
    ///   4. `[]` Timelock program account pub key.
    ///   5. `[]` Token program account.
    InitTimelockConfig {
        /// Consensus Algorithm
        consensus_algorithm: u8,
        /// Execution type
        execution_type: u8,
        /// Timelock Type
        timelock_type: u8,
        /// Voting entry rule
        voting_entry_rule: u8,
        /// Minimum slot time-distance from creation of proposal for an instruction to be placed
        minimum_slot_waiting_period: u64,
        /// Time limit in slots for proposal to be open to voting
        time_limit: u64,
        /// Optional name
        name: [u8; CONFIG_NAME_LENGTH],
    },

    ///   0. `[writable]` Timelock config key. Needs to be set with pubkey set to PDA with seeds of the
    ///           program account key, governance mint key, council mint key, and timelock program account key.
    ///   1. `[]` Program account to tie this config to.
    ///   2. `[]` Governance mint to tie this config to
    ///   3. `[]` Council mint [optional] to tie this config to [Pass in 0s otherwise]
    ///   4. `[]` Payer
    ///   5. `[]` Timelock program account pub key.
    ///   6. `[]` Timelock program pub key. Different from program account - is the actual id of the executable.
    ///   7. `[]` Token program account.
    ///   8. `[]` System account.
    CreateEmptyTimelockConfig,
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
                let (input_desc_link, input_name) = rest.split_at(DESC_SIZE);
                let mut desc_link: [u8; DESC_SIZE] = [0; DESC_SIZE];
                let mut name: [u8; NAME_SIZE] = [0; NAME_SIZE];
                for n in 0..(DESC_SIZE - 1) {
                    desc_link[n] = input_desc_link[n];
                }

                for n in 0..(NAME_SIZE - 1) {
                    name[n] = input_name[n];
                }
                Self::InitTimelockSet { desc_link, name }
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
                let (yes_voting_token_amount, rest) = Self::unpack_u64(rest)?;
                let (no_voting_token_amount, _) = Self::unpack_u64(rest)?;

                Self::Vote {
                    yes_voting_token_amount,
                    no_voting_token_amount,
                }
            }

            10 => {
                let (consensus_algorithm, rest) = Self::unpack_u8(rest)?;
                let (execution_type, rest) = Self::unpack_u8(rest)?;
                let (timelock_type, rest) = Self::unpack_u8(rest)?;
                let (voting_entry_rule, rest) = Self::unpack_u8(rest)?;
                let (minimum_slot_waiting_period, rest) = Self::unpack_u64(rest)?;
                let (time_limit, rest) = Self::unpack_u64(rest)?;
                let mut name: [u8; CONFIG_NAME_LENGTH] = [0; CONFIG_NAME_LENGTH];
                for n in 0..(CONFIG_NAME_LENGTH - 1) {
                    name[n] = rest[n];
                }
                Self::InitTimelockConfig {
                    consensus_algorithm,
                    execution_type,
                    timelock_type,
                    voting_entry_rule,
                    minimum_slot_waiting_period,
                    name,
                    time_limit,
                }
            }
            11 => Self::Ping,
            12 => {
                let (number_of_extra_accounts, _) = Self::unpack_u8(rest)?;
                Self::Execute {
                    number_of_extra_accounts,
                }
            }
            13 => {
                let (voting_token_amount, _) = Self::unpack_u64(rest)?;
                Self::DepositSourceTokens {
                    voting_token_amount,
                }
            }
            14 => {
                let (voting_token_amount, _) = Self::unpack_u64(rest)?;
                Self::WithdrawVotingTokens {
                    voting_token_amount,
                }
            }
            15 => Self::CreateEmptyTimelockConfig,
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
            Self::InitTimelockSet { desc_link, name } => {
                buf.push(1);
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
                yes_voting_token_amount,
                no_voting_token_amount,
            } => {
                buf.push(9);
                buf.extend_from_slice(&yes_voting_token_amount.to_le_bytes());
                buf.extend_from_slice(&no_voting_token_amount.to_le_bytes());
            }
            Self::InitTimelockConfig {
                consensus_algorithm,
                execution_type,
                timelock_type,
                voting_entry_rule,
                minimum_slot_waiting_period,
                time_limit,
                name,
            } => {
                buf.push(10);
                buf.extend_from_slice(&consensus_algorithm.to_le_bytes());
                buf.extend_from_slice(&execution_type.to_le_bytes());
                buf.extend_from_slice(&timelock_type.to_le_bytes());
                buf.extend_from_slice(&voting_entry_rule.to_le_bytes());
                buf.extend_from_slice(&minimum_slot_waiting_period.to_le_bytes());
                buf.extend_from_slice(&time_limit.to_le_bytes());
                buf.extend_from_slice(name);
            }
            Self::Ping => buf.push(11),
            Self::Execute {
                number_of_extra_accounts,
            } => {
                buf.push(12);
                buf.extend_from_slice(&number_of_extra_accounts.to_le_bytes());
            }
            Self::DepositSourceTokens {
                voting_token_amount,
            } => {
                buf.push(13);
                buf.extend_from_slice(&voting_token_amount.to_le_bytes());
            }
            Self::WithdrawVotingTokens {
                voting_token_amount,
            } => {
                buf.push(14);
                buf.extend_from_slice(&voting_token_amount.to_le_bytes());
            }
            Self::CreateEmptyTimelockConfig => buf.push(15),
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
