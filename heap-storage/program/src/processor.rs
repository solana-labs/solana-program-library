//! Program state processor

use crate::{
    error::HeapProgramError,
    instruction::HeapInstruction,
    state::{Heap, Node, EMPTY_NODE_DATA, HEAP_VERSION, ROOT_NODE_INDEX},
};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::next_account_info,
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    msg,
    program::invoke_signed,
    program_error::ProgramError,
    pubkey::Pubkey,
    system_instruction,
    sysvar::{rent::Rent, Sysvar},
};
use std::mem;

/// Program state handler.
pub struct Processor {}
impl Processor {
    /// Init new Heap
    pub fn process_init_heap(_program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let heap_account_info = next_account_info(account_info_iter)?;
        let authority_account_info = next_account_info(account_info_iter)?;
        let rent_account_info = next_account_info(account_info_iter)?;
        let rent = &Rent::from_account_info(rent_account_info)?;

        let mut heap = Heap::try_from_slice(&heap_account_info.data.borrow())?;
        if heap.is_initialized() {
            return Err(ProgramError::AccountAlreadyInitialized);
        }

        if !rent.is_exempt(heap_account_info.lamports(), heap_account_info.data_len()) {
            return Err(ProgramError::AccountNotRentExempt);
        }

        if !authority_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        heap.version = HEAP_VERSION;
        heap.authority = *authority_account_info.key;
        heap.size = 0;

        heap.serialize(&mut *heap_account_info.data.borrow_mut())
            .map_err(|e| e.into())
    }

    /// Add Node to Heap
    pub fn process_add_node(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        data: [u8; 32],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let heap_account_info = next_account_info(account_info_iter)?;
        let node_account_info = next_account_info(account_info_iter)?;
        let authority_account_info = next_account_info(account_info_iter)?;
        let rent_account_info = next_account_info(account_info_iter)?;
        let rent = &Rent::from_account_info(rent_account_info)?;

        let mut heap = Heap::try_from_slice(&heap_account_info.data.borrow())?;
        if !heap.is_initialized() {
            return Err(ProgramError::UninitializedAccount);
        }

        let mut node = Node::try_from_slice(&node_account_info.data.borrow())?;
        if node.is_initialized() {
            return Err(ProgramError::AccountAlreadyInitialized);
        }

        if !rent.is_exempt(node_account_info.lamports(), node_account_info.data_len()) {
            return Err(ProgramError::AccountNotRentExempt);
        }

        let (generated_node_address, _) = Pubkey::find_program_address(
            &[
                &heap_account_info.key.to_bytes()[..32],
                &heap.size.to_le_bytes(),
            ],
            program_id,
        );
        if generated_node_address != *node_account_info.key {
            return Err(HeapProgramError::WrongNodeAccount.into());
        }

        if *authority_account_info.key != heap.authority {
            return Err(HeapProgramError::WrongAuthority.into());
        }

        if !authority_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        if data == EMPTY_NODE_DATA {
            return Err(HeapProgramError::InvalidNodesData.into());
        }

        node.version = HEAP_VERSION;
        node.index = heap.size;
        node.data = data;
        heap.size = heap
            .size
            .checked_add(1)
            .ok_or_else::<ProgramError, _>(|| HeapProgramError::CalculationError.into())?;

        node.serialize(&mut *node_account_info.data.borrow_mut())?;
        heap.serialize(&mut *heap_account_info.data.borrow_mut())
            .map_err(|e| e.into())
    }

    /// Remove Node from the Heap
    pub fn process_remove_node(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let heap_account_info = next_account_info(account_info_iter)?;
        let root_node_account_info = next_account_info(account_info_iter)?;
        let leaf_node_account_info = next_account_info(account_info_iter)?;
        let authority_account_info = next_account_info(account_info_iter)?;

        let mut heap = Heap::try_from_slice(&heap_account_info.data.borrow())?;
        if !heap.is_initialized() {
            return Err(ProgramError::UninitializedAccount);
        }

        let mut root_node = Node::try_from_slice(&root_node_account_info.data.borrow())?;
        if !root_node.is_initialized() {
            return Err(ProgramError::UninitializedAccount);
        }

        let (generated_root_node_address, _) = Pubkey::find_program_address(
            &[
                &heap_account_info.key.to_bytes()[..32],
                &(ROOT_NODE_INDEX as u128).to_le_bytes(),
            ],
            program_id,
        );
        if generated_root_node_address != *root_node_account_info.key {
            return Err(HeapProgramError::WrongNodeAccount.into());
        }

        let mut leaf_node = Node::try_from_slice(&leaf_node_account_info.data.borrow())?;
        if !leaf_node.is_initialized() {
            return Err(ProgramError::UninitializedAccount);
        }

        let leaf_index = heap
            .size
            .checked_sub(1)
            .ok_or_else::<ProgramError, _>(|| HeapProgramError::CalculationError.into())?;
        let (generated_leaf_node_address, _) = Pubkey::find_program_address(
            &[
                &heap_account_info.key.to_bytes()[..32],
                &leaf_index.to_le_bytes(),
            ],
            program_id,
        );
        if generated_leaf_node_address != *leaf_node_account_info.key {
            return Err(HeapProgramError::WrongNodeAccount.into());
        }

        if *authority_account_info.key != heap.authority {
            return Err(HeapProgramError::WrongAuthority.into());
        }

        if !authority_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        if root_node_account_info.key == leaf_node_account_info.key {
            root_node = Node::default();
            root_node.serialize(&mut *root_node_account_info.data.borrow_mut())?;
        } else {
            root_node.data = leaf_node.data;
            leaf_node = Node::default();
            root_node.serialize(&mut *root_node_account_info.data.borrow_mut())?;
            leaf_node.serialize(&mut *leaf_node_account_info.data.borrow_mut())?;
        }

        heap.size = heap
            .size
            .checked_sub(1)
            .ok_or_else::<ProgramError, _>(|| HeapProgramError::CalculationError.into())?;

        heap.serialize(&mut *heap_account_info.data.borrow_mut())
            .map_err(|e| e.into())
    }

    /// Swap two nodes
    pub fn process_swap(_program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let heap_account_info = next_account_info(account_info_iter)?;
        let parent_node_account_info = next_account_info(account_info_iter)?;
        let child_node_account_info = next_account_info(account_info_iter)?;
        let authority_account_info = next_account_info(account_info_iter)?;

        let heap = Heap::try_from_slice(&heap_account_info.data.borrow())?;
        if !heap.is_initialized() {
            return Err(ProgramError::UninitializedAccount);
        }

        let mut parent_node = Node::try_from_slice(&parent_node_account_info.data.borrow())?;
        if !parent_node.is_initialized() {
            return Err(ProgramError::UninitializedAccount);
        }

        let mut child_node = Node::try_from_slice(&child_node_account_info.data.borrow())?;
        if !child_node.is_initialized() {
            return Err(ProgramError::UninitializedAccount);
        }

        if parent_node.index > heap.size || child_node.index > heap.size {
            return Err(HeapProgramError::NodeIndexesOutOfRange.into());
        }

        let parent_index = child_node
            .index
            .checked_sub(1)
            .ok_or_else::<ProgramError, _>(|| HeapProgramError::CalculationError.into())?
            .checked_div(2)
            .ok_or_else::<ProgramError, _>(|| HeapProgramError::CalculationError.into())?;
        if parent_index != parent_node.index {
            return Err(HeapProgramError::NodesAreNotRelated.into());
        }

        if *authority_account_info.key != heap.authority {
            return Err(HeapProgramError::WrongAuthority.into());
        }

        if !authority_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        mem::swap(&mut parent_node.data, &mut child_node.data);

        parent_node.serialize(&mut *parent_node_account_info.data.borrow_mut())?;
        child_node
            .serialize(&mut *child_node_account_info.data.borrow_mut())
            .map_err(|e| e.into())
    }

    /// Create new Node account
    pub fn process_create_node_account(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let payer_account_info = next_account_info(account_info_iter)?;
        let heap_account_info = next_account_info(account_info_iter)?;
        let account_to_create_account_info = next_account_info(account_info_iter)?;
        let rent_account_info = next_account_info(account_info_iter)?;
        let rent = &Rent::from_account_info(rent_account_info)?;
        // Need in System Program account because we call create_account_with_seed instruction which requires it
        let _system_program = next_account_info(account_info_iter)?;

        let heap = Heap::try_from_slice(&heap_account_info.data.borrow())?;
        if !heap.is_initialized() {
            return Err(ProgramError::UninitializedAccount);
        }

        let (generated_address, bump_seed) = Pubkey::find_program_address(
            &[
                &heap_account_info.key.to_bytes()[..32],
                &heap.size.to_le_bytes(),
            ],
            program_id,
        );
        if generated_address != *account_to_create_account_info.key {
            return Err(ProgramError::InvalidSeeds);
        }

        let signature = &[
            &heap_account_info.key.to_bytes()[..32],
            &heap.size.to_le_bytes(),
            &[bump_seed],
        ];

        invoke_signed(
            &system_instruction::create_account(
                payer_account_info.key,
                account_to_create_account_info.key,
                rent.minimum_balance(Node::LEN),
                Node::LEN as u64,
                program_id,
            ),
            &[
                payer_account_info.clone(),
                account_to_create_account_info.clone(),
            ],
            &[signature],
        )
    }

    /// Processes an instruction
    pub fn process_instruction(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        input: &[u8],
    ) -> ProgramResult {
        let instruction = HeapInstruction::try_from_slice(input)?;
        match instruction {
            HeapInstruction::InitHeap => {
                msg!("Instruction: InitHeap");
                Self::process_init_heap(program_id, accounts)
            }
            HeapInstruction::AddNode(data) => {
                msg!("Instruction: AddNode");
                Self::process_add_node(program_id, accounts, data)
            }
            HeapInstruction::RemoveNode => {
                msg!("Instruction: RemoveNode");
                Self::process_remove_node(program_id, accounts)
            }
            HeapInstruction::Swap => {
                msg!("Instruction: Swap");
                Self::process_swap(program_id, accounts)
            }
            HeapInstruction::CreateNodeAccount => {
                msg!("Instruction: CreateNodeAccount");
                Self::process_create_node_account(program_id, accounts)
            }
        }
    }
}
