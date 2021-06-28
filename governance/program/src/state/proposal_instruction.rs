//! ProposalInstruction Account

use crate::{
    error::GovernanceError,
    state::enums::GovernanceAccountType,
    tools::account::{get_account_data, AccountMaxSize},
    PROGRAM_AUTHORITY_SEED,
};
use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use solana_program::{
    account_info::AccountInfo,
    clock::UnixTimestamp,
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

    /// Minimum waiting time in seconds for the  instruction to be executed once proposal is voted on
    pub hold_up_time: u32,

    /// Instruction to execute
    /// The instruction will be signed by Governance PDA the Proposal belongs to
    // For example for ProgramGovernance the instruction to upgrade program will be signed by ProgramGovernance PDA
    pub instruction: InstructionData,

    /// Executed at flag
    pub executed_at: Option<UnixTimestamp>,
}

impl AccountMaxSize for ProposalInstruction {
    fn get_max_size(&self) -> Option<usize> {
        Some(self.instruction.accounts.len() * 34 + self.instruction.data.len() + 86)
    }
}

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
        instruction_index_le_bytes,
    ]
}

/// Returns ProposalInstruction PDA address
pub fn get_proposal_instruction_address<'a>(
    program_id: &Pubkey,
    proposal: &'a Pubkey,
    instruction_index_le_bytes: &'a [u8],
) -> Pubkey {
    Pubkey::find_program_address(
        &get_proposal_instruction_address_seeds(proposal, instruction_index_le_bytes),
        program_id,
    )
    .0
}

/// Deserializes ProposalInstruction account and checks owner program
pub fn get_proposal_instruction_data(
    program_id: &Pubkey,
    proposal_instruction_info: &AccountInfo,
) -> Result<ProposalInstruction, ProgramError> {
    get_account_data::<ProposalInstruction>(proposal_instruction_info, program_id)
}

///  Deserializes and returns ProposalInstruction account and checks it belongs to the given Proposal
pub fn get_proposal_instruction_data_for_proposal(
    program_id: &Pubkey,
    proposal_instruction_info: &AccountInfo,
    proposal: &Pubkey,
) -> Result<ProposalInstruction, ProgramError> {
    let proposal_instruction_data =
        get_proposal_instruction_data(program_id, proposal_instruction_info)?;

    if proposal_instruction_data.proposal != *proposal {
        return Err(GovernanceError::InvalidProposalForProposalInstruction.into());
    }

    Ok(proposal_instruction_data)
}

///  Deserializes ProposalInstruction account and checks it belongs to the given Proposal
pub fn assert_proposal_instruction_for_proposal(
    program_id: &Pubkey,
    proposal_instruction_info: &AccountInfo,
    proposal: &Pubkey,
) -> Result<(), ProgramError> {
    get_proposal_instruction_data_for_proposal(program_id, proposal_instruction_info, proposal)
        .map(|_| ())
}

#[cfg(test)]
mod test {

    use super::*;

    fn create_test_account_meta_data() -> AccountMetaData {
        AccountMetaData {
            pubkey: Pubkey::new_unique(),
            is_signer: true,
            is_writable: false,
        }
    }

    fn create_test_instruction_data() -> InstructionData {
        InstructionData {
            program_id: Pubkey::new_unique(),
            accounts: vec![
                create_test_account_meta_data(),
                create_test_account_meta_data(),
            ],
            data: vec![1, 2, 3],
        }
    }

    fn create_test_proposal_instruction() -> ProposalInstruction {
        ProposalInstruction {
            account_type: GovernanceAccountType::ProposalInstruction,
            proposal: Pubkey::new_unique(),
            hold_up_time: 10,
            instruction: create_test_instruction_data(),
            executed_at: Some(100),
        }
    }

    #[test]
    fn test_account_meta_data_size() {
        let account_meta_data = create_test_account_meta_data();
        let size = account_meta_data.try_to_vec().unwrap().len();

        assert_eq!(34, size);
    }

    #[test]
    fn test_proposal_instruction_max_size() {
        // Arrange
        let proposal_instruction = create_test_proposal_instruction();
        let size = proposal_instruction.try_to_vec().unwrap().len();

        // Act, Assert
        assert_eq!(proposal_instruction.get_max_size(), Some(size));
    }

    #[test]
    fn test_empty_proposal_instruction_max_size() {
        // Arrange
        let mut proposal_instruction = create_test_proposal_instruction();
        proposal_instruction.instruction.data = vec![];
        proposal_instruction.instruction.accounts = vec![];

        let size = proposal_instruction.try_to_vec().unwrap().len();

        // Act, Assert
        assert_eq!(proposal_instruction.get_max_size(), Some(size));
    }
}
