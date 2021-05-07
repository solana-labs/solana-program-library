//! Instruction types

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    instruction::{AccountMeta, Instruction},
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvar,
};
use crate::processor::Processor;

/// input
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct Add {
    /// Amount
    pub amount: u8,
}

/// Instruction definition
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub enum ExampleInstruction {
    /// InitStorage
    ///
    ///   0. `[w]` Uninitialized heap account
    ///   1. `[r]` Rent
    InitStorage,

    /// Add
    ///
    ///   0. `[w]` Uninitialized account
    ///   1. `[w]` Created node account
    ///   2. `[w]` Heap account
    ///   3. `[r]` Rent
    Add(Add),

    /// Remove
    ///
    ///   0. `[w]` Data account
    ///   1. `[w]` Heap account
    ///   2. `[w]` Node account
    ///   3. `[w]` Leaf account
    Remove,

    /// Sort
    ///
    ///   0. `[w]` Parent node
    ///   1. `[r]` Parent nodes data acc
    ///   2. `[w]` Child node
    ///   3. `[r]` Child nodes data acc
    ///   4. `[]` Heap account
    Sort,
}

/// Create `InitStorage` instruction
pub fn init(program_id: &Pubkey, heap: &Pubkey) -> Result<Instruction, ProgramError> {
    let init_data = ExampleInstruction::InitStorage;
    let data = init_data.try_to_vec()?;
    let authority = Processor::get_authority(heap).0;
    let accounts = vec![
        AccountMeta::new(*heap, false),
        AccountMeta::new_readonly(authority, false),
        AccountMeta::new_readonly(heap_storage::id(), false),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
    ];
    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data,
    })
}

/// Create `Add` instruction
pub fn add(
    program_id: &Pubkey,
    data_acc: &Pubkey,
    node: &Pubkey,
    heap: &Pubkey,
    input: Add,
) -> Result<Instruction, ProgramError> {
    let init_data = ExampleInstruction::Add(input);
    let data = init_data.try_to_vec()?;
    let authority = Processor::get_authority(heap).0;
    let accounts = vec![
        AccountMeta::new(*data_acc, false),
        AccountMeta::new(*node, false),
        AccountMeta::new(*heap, false),
        AccountMeta::new_readonly(authority, false),
        AccountMeta::new_readonly(heap_storage::id(), false),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
    ];
    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data,
    })
}

/// Create `Remove` instruction
pub fn remove(
    program_id: &Pubkey,
    data_acc: &Pubkey,
    heap: &Pubkey,
    node: &Pubkey,
    leaf: &Pubkey,
) -> Result<Instruction, ProgramError> {
    let init_data = ExampleInstruction::Remove;
    let data = init_data.try_to_vec()?;
    let authority = Processor::get_authority(heap).0;
    let accounts = vec![
        AccountMeta::new(*data_acc, false),
        AccountMeta::new(*heap, false),
        AccountMeta::new(*node, false),
        AccountMeta::new(*leaf, false),
        AccountMeta::new_readonly(authority, false),
    ];
    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data,
    })
}

/// Create `Sort` instruction
pub fn sort(
    program_id: &Pubkey,
    parent_node: &Pubkey,
    parent_node_data: &Pubkey,
    child_node: &Pubkey,
    child_node_data: &Pubkey,
    heap: &Pubkey,
) -> Result<Instruction, ProgramError> {
    let init_data = ExampleInstruction::Sort;
    let data = init_data.try_to_vec()?;
    let authority = Processor::get_authority(heap).0;
    let accounts = vec![
        AccountMeta::new(*parent_node, false),
        AccountMeta::new_readonly(*parent_node_data, false),
        AccountMeta::new(*child_node, false),
        AccountMeta::new_readonly(*child_node_data, false),
        AccountMeta::new_readonly(authority, false),
    ];
    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data,
    })
}
