use crate::state::node::{Node, EMPTY};
use anchor_lang::{
    prelude::*,
    solana_program::{keccak::hashv, msg, program_error::ProgramError},
};
use bytemuck::{Pod, PodCastError};
use std::any::type_name;
use std::mem::size_of;

pub trait ZeroCopy: Pod {
    fn load_mut_bytes<'a>(data: &'a mut [u8]) -> Result<&'a mut Self> {
        let size = size_of::<Self>();
        let data_len = data.len();

        Ok(bytemuck::try_from_bytes_mut(&mut data[..size])
            .map_err(error_msg::<Self>(data_len))
            .unwrap())
    }
}

/// Calculates hash of empty nodes up to level i
pub fn empty_node(level: u32) -> Node {
    let mut data = EMPTY;
    if level != 0 {
        let lower_empty = empty_node(level - 1);
        let hash = hashv(&[lower_empty.as_ref(), lower_empty.as_ref()]);
        data.copy_from_slice(hash.as_ref());
    }
    data
}

/// Recomputes root of the Merkle tree from Node & proof
pub fn recompute(leaf: Node, proof: &[Node], index: u32) -> Node {
    let mut current_node = leaf;
    for (depth, sibling_leaf) in proof.iter().enumerate() {
        if index >> depth & 1 == 0 {
            let res = hashv(&[current_node.as_ref(), sibling_leaf.as_ref()]);
            current_node.copy_from_slice(res.as_ref());
        } else {
            let res = hashv(&[sibling_leaf.as_ref(), current_node.as_ref()]);
            current_node.copy_from_slice(res.as_ref());
        }
    }

    current_node
}

pub fn fill_in_proof<const MAX_DEPTH: usize>(
    proof_vec: Vec<Node>,
    full_proof: &mut [Node; MAX_DEPTH],
) {
    msg!("Attempting to fill in proof");
    if proof_vec.len() > 0 {
        full_proof[..proof_vec.len()].copy_from_slice(&proof_vec);
    }

    for i in proof_vec.len()..MAX_DEPTH {
        full_proof[i] = empty_node(i as u32);
    }
}

pub fn error_msg<T>(data_len: usize) -> impl Fn(PodCastError) -> ProgramError {
    move |_: PodCastError| -> ProgramError {
        msg!(
            "Failed to load {}. Size is {}, expected {}",
            type_name::<T>(),
            data_len,
            size_of::<T>(),
        );
        ProgramError::InvalidAccountData
    }
}
