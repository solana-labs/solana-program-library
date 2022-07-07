use crate::state::{Node, EMPTY};
use solana_program::{keccak::hashv, msg};

/// Calculates hash of empty nodes up to level i
pub fn empty_node(level: u32) -> Node {
    empty_node_cached::<0>(level, &mut Box::new([]))
}

/// Calculates hash of empty nodes up to level i
pub fn empty_node_cached<const N: usize>(level: u32, cache: &mut Box<[Node; N]>) -> Node {
    let mut data = EMPTY;
    if level != 0 {
        let target = (level - 1) as usize;
        let lower_empty = if target < cache.len() && cache[target] != EMPTY {
            cache[target]
        } else {
            empty_node(target as u32)
        };
        let hash = hashv(&[lower_empty.as_ref(), lower_empty.as_ref()]);
        data.copy_from_slice(hash.as_ref());
    }
    data
}

/// Recomputes root of the Merkle tree from Node & proof
pub fn recompute(leaf: Node, proof: &[Node], index: u32) -> Node {
    let mut current_node = leaf;
    for (depth, sibling) in proof.iter().enumerate() {
        hash_to_parent(&mut current_node, sibling, index >> depth & 1 == 0);
    }
    current_node
}

/// Computes the parent node of `node` and `sibling` and copies the result into `node`
#[inline(always)]
pub fn hash_to_parent(node: &mut Node, sibling: &Node, is_left: bool) {
    let parent = if is_left {
        hashv(&[node, sibling])
    } else {
        hashv(&[sibling, node])
    };
    node.copy_from_slice(parent.as_ref())
}

pub fn fill_in_proof<const MAX_DEPTH: usize>(
    proof_vec: &Vec<Node>,
    full_proof: &mut [Node; MAX_DEPTH],
) {
    solana_logging!("Attempting to fill in proof");
    if proof_vec.len() > 0 {
        full_proof[..proof_vec.len()].copy_from_slice(&proof_vec);
    }

    for i in proof_vec.len()..MAX_DEPTH {
        full_proof[i] = empty_node(i as u32);
    }
}
