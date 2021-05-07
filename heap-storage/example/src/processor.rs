//! Program state processor

use crate::{error::ProgramTemplateError, instruction::{ExampleInstruction, Add}};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::next_account_info, account_info::AccountInfo, entrypoint::ProgramResult, msg,
    pubkey::Pubkey,
    sysvar::rent::Rent,
    sysvar::Sysvar,
};

/// Program state handler.
pub struct Processor {}
impl Processor {
    /// Create storage
    pub fn process_init_storage(
        _program_id: &Pubkey,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let heap_account_info = next_account_info(account_info_iter)?;
        let authority_account_info = next_account_info(account_info_iter)?;
        let rent_info = next_account_info(account_info_iter)?;
        let rent = &Rent::from_account_info(rent_info)?;

        // call heap-program to InitHeap
        // * we init heap through the program because we set authority as program address *

        Ok(())
    }

    /// Add new node
    pub fn process_add(
        _program_id: &Pubkey,
        accounts: &[AccountInfo],
        input: Add,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let data_account_info = next_account_info(account_info_iter)?;
        let node_account_info = next_account_info(account_info_iter)?;
        let heap_account_info = next_account_info(account_info_iter)?;
        let authority_account_info = next_account_info(account_info_iter)?;
        let rent_info = next_account_info(account_info_iter)?;
        let rent = &Rent::from_account_info(rent_info)?;

        // check if data_account_info is rent exempt
        // call heap-program to AddNode

        Ok(())
    }

    /// Remove node
    pub fn process_remove(
        _program_id: &Pubkey,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let data_account_info = next_account_info(account_info_iter)?;
        let heap_account_info = next_account_info(account_info_iter)?;
        let node_account_info = next_account_info(account_info_iter)?;
        let leaf_account_info = next_account_info(account_info_iter)?;
        let authority_account_info = next_account_info(account_info_iter)?;

        // call heap-program RemoveNode
        // clean data_accounts

        Ok(())
    }

    /// Sort data
    pub fn process_sort(
        _program_id: &Pubkey,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let parent_node_acc_info = next_account_info(account_info_iter)?;
        let parent_node_data_acc_info = next_account_info(account_info_iter)?;
        let child_node_acc_info = next_account_info(account_info_iter)?;
        let child_node_data_acc_info = next_account_info(account_info_iter)?;
        let authority_account_info = next_account_info(account_info_iter)?;

        // check that parent_node_acc_info data is parent_node_data_acc_info address
        // check that child_node_acc_info data is child_node_data_acc_info address
        // check that value of child_node_data_acc_info is less then value of parent_node_data_acc_info
        // call heap-program Swap

        Ok(())
    }

    /// Processes an instruction
    pub fn process_instruction(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        input: &[u8],
    ) -> ProgramResult {
        let instruction = ExampleInstruction::try_from_slice(input)?;
        match instruction {
            ExampleInstruction::InitStorage => {
                msg!("Instruction: InitStorage");
                unimplemented!()
            }
            ExampleInstruction::Add(input) => {
                msg!("Instruction: Add");
                unimplemented!()
            }
            ExampleInstruction::Sort => {
                msg!("Instruction: Sort");
                unimplemented!()
            }
            ExampleInstruction::Remove => {
                msg!("Instruction: Remove");
                unimplemented!()
            }
        }
    }
}
