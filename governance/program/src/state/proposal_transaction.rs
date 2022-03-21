//! ProposalTransaction Account

use core::panic;

use borsh::maybestd::io::Write;

use crate::{
    error::GovernanceError,
    state::{
        enums::{GovernanceAccountType, TransactionExecutionStatus},
        legacy::ProposalInstructionV1,
    },
    PROGRAM_AUTHORITY_SEED,
};
use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use solana_program::{
    account_info::AccountInfo,
    borsh::try_from_slice_unchecked,
    clock::UnixTimestamp,
    instruction::{AccountMeta, Instruction},
    program_error::ProgramError,
    program_pack::IsInitialized,
    pubkey::Pubkey,
};
use spl_governance_tools::account::{get_account_data, AccountMaxSize};

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
pub struct ProposalTransactionV2 {
    /// Governance Account type
    pub account_type: GovernanceAccountType,

    /// The Proposal the instruction belongs to
    pub proposal: Pubkey,

    /// The option index the instruction belongs to
    pub option_index: u8,

    /// Unique transaction index within it's parent Proposal
    pub transaction_index: u16,

    /// Minimum waiting time in seconds for the  instruction to be executed once proposal is voted on
    pub hold_up_time: u32,

    /// Instructions to execute
    /// The instructions will be signed by Governance PDA the Proposal belongs to
    // For example for ProgramGovernance the instruction to upgrade program will be signed by ProgramGovernance PDA
    // All instructions will be executed within a single transaction
    pub instructions: Vec<InstructionData>,

    /// Executed at flag
    pub executed_at: Option<UnixTimestamp>,

    /// Instruction execution status
    pub execution_status: TransactionExecutionStatus,

    /// Reserved space for versions v2 and onwards
    /// Note: This space won't be available to v1 accounts until runtime supports resizing
    pub reserved_v2: [u8; 8],
}

impl AccountMaxSize for ProposalTransactionV2 {
    fn get_max_size(&self) -> Option<usize> {
        let instructions_size = self
            .instructions
            .iter()
            .map(|i| i.accounts.len() * 34 + i.data.len() + 40)
            .sum::<usize>();

        Some(instructions_size + 62)
    }
}

impl IsInitialized for ProposalTransactionV2 {
    fn is_initialized(&self) -> bool {
        self.account_type == GovernanceAccountType::ProposalTransactionV2
    }
}

impl ProposalTransactionV2 {
    /// Serializes account into the target buffer
    pub fn serialize<W: Write>(self, writer: &mut W) -> Result<(), ProgramError> {
        if self.account_type == GovernanceAccountType::ProposalTransactionV2 {
            BorshSerialize::serialize(&self, writer)?
        } else if self.account_type == GovernanceAccountType::ProposalInstructionV1 {
            if self.instructions.len() != 1 {
                panic!("Multiple instructions are not supported by ProposalInstructionV1")
            };

            // V1 account can't be resized and we have to translate it back to the original format

            // If reserved_v2 is used it must be individually asses for v1 backward compatibility impact
            if self.reserved_v2 != [0; 8] {
                panic!("Extended data not supported by ProposalInstructionV1")
            }

            let proposal_transaction_data_v1 = ProposalInstructionV1 {
                account_type: self.account_type,
                proposal: self.proposal,
                instruction_index: self.transaction_index,
                hold_up_time: self.hold_up_time,
                instruction: self.instructions[0].clone(),
                executed_at: self.executed_at,
                execution_status: self.execution_status,
            };

            BorshSerialize::serialize(&proposal_transaction_data_v1, writer)?;
        }

        Ok(())
    }
}

/// Returns ProposalTransaction PDA seeds
pub fn get_proposal_transaction_address_seeds<'a>(
    proposal: &'a Pubkey,
    option_index: &'a [u8; 1],               // u8 le bytes
    instruction_index_le_bytes: &'a [u8; 2], // u16 le bytes
) -> [&'a [u8]; 4] {
    [
        PROGRAM_AUTHORITY_SEED,
        proposal.as_ref(),
        option_index,
        instruction_index_le_bytes,
    ]
}

/// Returns ProposalTransaction PDA address
pub fn get_proposal_transaction_address<'a>(
    program_id: &Pubkey,
    proposal: &'a Pubkey,
    option_index_le_bytes: &'a [u8; 1],      // u8 le bytes
    instruction_index_le_bytes: &'a [u8; 2], // u16 le bytes
) -> Pubkey {
    Pubkey::find_program_address(
        &get_proposal_transaction_address_seeds(
            proposal,
            option_index_le_bytes,
            instruction_index_le_bytes,
        ),
        program_id,
    )
    .0
}

/// Deserializes ProposalTransaction account and checks owner program
pub fn get_proposal_transaction_data(
    program_id: &Pubkey,
    proposal_transaction_info: &AccountInfo,
) -> Result<ProposalTransactionV2, ProgramError> {
    let account_type: GovernanceAccountType =
        try_from_slice_unchecked(&proposal_transaction_info.data.borrow())?;

    // If the account is V1 version then translate to V2
    if account_type == GovernanceAccountType::ProposalInstructionV1 {
        let proposal_transaction_data_v1 =
            get_account_data::<ProposalInstructionV1>(program_id, proposal_transaction_info)?;

        return Ok(ProposalTransactionV2 {
            account_type,
            proposal: proposal_transaction_data_v1.proposal,
            option_index: 0, // V1 has a single implied option at index 0
            transaction_index: proposal_transaction_data_v1.instruction_index,
            hold_up_time: proposal_transaction_data_v1.hold_up_time,
            instructions: vec![proposal_transaction_data_v1.instruction],
            executed_at: proposal_transaction_data_v1.executed_at,
            execution_status: proposal_transaction_data_v1.execution_status,
            reserved_v2: [0; 8],
        });
    }

    get_account_data::<ProposalTransactionV2>(program_id, proposal_transaction_info)
}

///  Deserializes and returns ProposalTransaction account and checks it belongs to the given Proposal
pub fn get_proposal_transaction_data_for_proposal(
    program_id: &Pubkey,
    proposal_transaction_info: &AccountInfo,
    proposal: &Pubkey,
) -> Result<ProposalTransactionV2, ProgramError> {
    let proposal_transaction_data =
        get_proposal_transaction_data(program_id, proposal_transaction_info)?;

    if proposal_transaction_data.proposal != *proposal {
        return Err(GovernanceError::InvalidProposalForProposalTransaction.into());
    }

    Ok(proposal_transaction_data)
}

#[cfg(test)]
mod test {

    use std::str::FromStr;

    use solana_program::{bpf_loader_upgradeable, clock::Epoch};

    use super::*;

    fn create_test_account_meta_data() -> AccountMetaData {
        AccountMetaData {
            pubkey: Pubkey::new_unique(),
            is_signer: true,
            is_writable: false,
        }
    }

    fn create_test_instruction_data() -> Vec<InstructionData> {
        vec![InstructionData {
            program_id: Pubkey::new_unique(),
            accounts: vec![
                create_test_account_meta_data(),
                create_test_account_meta_data(),
                create_test_account_meta_data(),
            ],
            data: vec![1, 2, 3],
        }]
    }

    fn create_test_proposal_transaction() -> ProposalTransactionV2 {
        ProposalTransactionV2 {
            account_type: GovernanceAccountType::ProposalTransactionV2,
            proposal: Pubkey::new_unique(),
            option_index: 0,
            transaction_index: 1,
            hold_up_time: 10,
            instructions: create_test_instruction_data(),
            executed_at: Some(100),
            execution_status: TransactionExecutionStatus::Success,
            reserved_v2: [0; 8],
        }
    }

    #[test]
    fn test_account_meta_data_size() {
        let account_meta_data = create_test_account_meta_data();
        let size = account_meta_data.try_to_vec().unwrap().len();

        assert_eq!(34, size);
    }

    #[test]
    fn test_proposal_transaction_max_size() {
        // Arrange
        let proposal_transaction = create_test_proposal_transaction();
        let size = proposal_transaction.try_to_vec().unwrap().len();

        // Act, Assert
        assert_eq!(proposal_transaction.get_max_size(), Some(size));
    }

    #[test]
    fn test_empty_proposal_transaction_max_size() {
        // Arrange
        let mut proposal_transaction = create_test_proposal_transaction();
        proposal_transaction.instructions[0].data = vec![];
        proposal_transaction.instructions[0].accounts = vec![];

        let size = proposal_transaction.try_to_vec().unwrap().len();

        // Act, Assert
        assert_eq!(proposal_transaction.get_max_size(), Some(size));
    }

    #[test]
    fn test_upgrade_instruction_serialization() {
        // Arrange
        let program_address =
            Pubkey::from_str("Hita5Lun87S4MADAF4vGoWEgFm5DyuVqxoWzzqYxS3AD").unwrap();
        let buffer_address =
            Pubkey::from_str("5XqXkgJGAUwrUHBkxbKpYMGqsRoQLfyqRbYUEkjNY6hL").unwrap();
        let governance = Pubkey::from_str("FqSReK9R8QxvFZgdrAwGT3gsYp1ZGfiFjS8xrzyyadn3").unwrap();

        let upgrade_instruction = bpf_loader_upgradeable::upgrade(
            &program_address,
            &buffer_address,
            &governance,
            &governance,
        );

        // Act
        let instruction_data: InstructionData = upgrade_instruction.clone().into();
        let mut instruction_bytes = vec![];
        instruction_data.serialize(&mut instruction_bytes).unwrap();

        // base64 encoded message is accepted as the input in the UI
        let base64 = base64::encode(instruction_bytes.clone());

        // Assert
        let instruction =
            Instruction::from(&InstructionData::deserialize(&mut &instruction_bytes[..]).unwrap());

        assert_eq!(upgrade_instruction, instruction);

        assert_eq!(base64,"Aqj2kU6IobDiEBU+92OuKwDCuT0WwSTSwFN6EASAAAAHAAAAchkHXTU9jF+rKpILT6dzsVyNI9NsQy9cab+GGvdwNn0AAfh2HVruy2YibpgcQUmJf5att5YdPXSv1k2pRAKAfpSWAAFDVQuXWos2urmegSPblI813GlTm7CJ/8rv+9yzNE3yfwAB3Gw+apCyfrRNqJ6f1160Htkx+uYZT6FIILQ3WzNA4KwAAQan1RcZLFxRIYzJTD1K8X9Y2u4Im6H9ROPb2YoAAAAAAAAGp9UXGMd0yShWY5hpHV62i164o5tLbVxzVVshAAAAAAAA3Gw+apCyfrRNqJ6f1160Htkx+uYZT6FIILQ3WzNA4KwBAAQAAAADAAAA");
    }

    #[test]
    fn test_proposal_transaction_v1_to_v2_serialization_roundtrip() {
        // Arrange

        let proposal_transaction_v1_source = ProposalInstructionV1 {
            account_type: GovernanceAccountType::ProposalInstructionV1,
            proposal: Pubkey::new_unique(),
            instruction_index: 1,
            hold_up_time: 120,
            instruction: create_test_instruction_data()[0].clone(),
            executed_at: Some(155),
            execution_status: TransactionExecutionStatus::Success,
        };

        let mut account_data = vec![];
        proposal_transaction_v1_source
            .serialize(&mut account_data)
            .unwrap();

        let program_id = Pubkey::new_unique();

        let info_key = Pubkey::new_unique();
        let mut lamports = 10u64;

        let account_info = AccountInfo::new(
            &info_key,
            false,
            false,
            &mut lamports,
            &mut account_data[..],
            &program_id,
            false,
            Epoch::default(),
        );

        // Act

        let proposal_transaction_v2 =
            get_proposal_transaction_data(&program_id, &account_info).unwrap();

        proposal_transaction_v2
            .serialize(&mut &mut **account_info.data.borrow_mut())
            .unwrap();

        // Assert
        let proposal_transaction_v1_target =
            get_account_data::<ProposalInstructionV1>(&program_id, &account_info).unwrap();

        assert_eq!(
            proposal_transaction_v1_source,
            proposal_transaction_v1_target
        )
    }
}
