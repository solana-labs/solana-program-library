//! SingleSignerInstruction Account

use crate::{
    error::GovernanceError,
    id,
    state::enums::GovernanceAccountType,
    tools::account::{get_account_data, AccountMaxSize},
    PROGRAM_AUTHORITY_SEED,
};
use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use solana_program::{
    account_info::AccountInfo,
    instruction::{AccountMeta, Instruction},
    program_error::ProgramError,
    program_pack::IsInitialized,
    pubkey::Pubkey,
};

/// InstructionData wrapper. It can be removed once Borsh serialization for Instruction is supported in the SDK
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
#[repr(C)]
pub struct InstructionData {
    /// Pubkey of the instruction processor that executes this instruction
    pub program_id: Pubkey,
    /// Metadata for what accounts should be passed to the instruction processor
    pub accounts: Vec<AccountMetaData>,
    /// Opaque data passed to the instruction processor
    pub data: Vec<u8>,
}

/// Account metadata used to define Instructions
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
#[repr(C)]
pub struct AccountMetaData {
    /// An account's public key
    pub pubkey: Pubkey,
    /// True if an Instruction requires a Transaction signature matching `pubkey`.
    pub is_signer: bool,
    /// True if the `pubkey` can be loaded as a read-write account.
    pub is_writable: bool,
}

impl From<Instruction> for InstructionData {
    fn from(instruction: Instruction) -> Self {
        InstructionData {
            program_id: instruction.program_id,
            accounts: instruction
                .accounts
                .iter()
                .map(|a| AccountMetaData {
                    pubkey: a.pubkey,
                    is_signer: a.is_signer,
                    is_writable: a.is_writable,
                })
                .collect(),
            data: instruction.data,
        }
    }
}

impl From<&InstructionData> for Instruction {
    fn from(instruction: &InstructionData) -> Self {
        Instruction {
            program_id: instruction.program_id,
            accounts: instruction
                .accounts
                .iter()
                .map(|a| AccountMeta {
                    pubkey: a.pubkey,
                    is_signer: a.is_signer,
                    is_writable: a.is_writable,
                })
                .collect(),
            data: instruction.data.clone(),
        }
    }
}

/// Account for an instruction to be executed for Proposal
#[repr(C)]
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub struct ProposalInstruction {
    /// Governance Account type
    pub account_type: GovernanceAccountType,

    /// The Proposal the instruction belongs to
    pub proposal: Pubkey,

    /// Minimum waiting time in slots for the  instruction to be executed once proposal is voted on
    pub hold_up_time: u64,

    /// Instruction to execute
    /// The instruction will be signed by Governance PDA the Proposal belongs to
    // For example for ProgramGovernance the instruction to upgrade program will be signed by ProgramGovernance PDA
    pub instruction: InstructionData,

    /// Executed flag
    pub executed: bool,
}

impl AccountMaxSize for ProposalInstruction {}

impl IsInitialized for ProposalInstruction {
    fn is_initialized(&self) -> bool {
        self.account_type == GovernanceAccountType::ProposalInstruction
    }
}

/// Returns ProposalInstruction PDA seeds
pub fn get_proposal_instruction_address_seeds<'a>(
    proposal: &'a Pubkey,
    instruction_index_le_bytes: &'a [u8],
) -> [&'a [u8]; 3] {
    [
        PROGRAM_AUTHORITY_SEED,
        proposal.as_ref(),
        &instruction_index_le_bytes,
    ]
}

/// Returns ProposalInstruction PDA address
pub fn get_proposal_instruction_address<'a>(
    proposal: &'a Pubkey,
    instruction_index_le_bytes: &'a [u8],
) -> Pubkey {
    Pubkey::find_program_address(
        &get_proposal_instruction_address_seeds(proposal, &instruction_index_le_bytes),
        &id(),
    )
    .0
}

/// Deserializes ProposalInstruction account and checks owner program
pub fn get_proposal_instruction_data(
    proposal_instruction_info: &AccountInfo,
) -> Result<ProposalInstruction, ProgramError> {
    get_account_data::<ProposalInstruction>(proposal_instruction_info, &id())
}

///  Deserializes and returns ProposalInstruction account and checks it belongs to the given Proposal
pub fn get_proposal_instruction_data_for_proposal(
    proposal_instruction_info: &AccountInfo,
    proposal: &Pubkey,
) -> Result<ProposalInstruction, ProgramError> {
    let proposal_instruction_data = get_proposal_instruction_data(proposal_instruction_info)?;

    if proposal_instruction_data.proposal != *proposal {
        return Err(GovernanceError::InvalidProposalForProposalInstruction.into());
    }

    Ok(proposal_instruction_data)
}

///  Deserializes ProposalInstruction account and checks it belongs to the given Proposal
pub fn assert_proposal_instruction_for_proposal(
    proposal_instruction_info: &AccountInfo,
    proposal: &Pubkey,
) -> Result<(), ProgramError> {
    get_proposal_instruction_data_for_proposal(proposal_instruction_info, proposal).map(|_| ())
}
