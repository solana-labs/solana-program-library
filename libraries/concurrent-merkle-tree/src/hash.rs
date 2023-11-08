use {
    crate::node::{empty_node, Node},
    solana_program::keccak::hashv,
};

/// Recomputes root of the Merkle tree from Node & proof
pub fn recompute(leaf: Node, proof: &[Node], index: u32) -> Node {
    let mut current_node = leaf;
    for (depth, sibling) in proof.iter().enumerate() {
        hash_to_parent(&mut current_node, sibling, index >> depth & 1 == 0);
    }
    current_node
}

/// Computes the parent node of `node` and `sibling` and copies the result into
/// `node`
#[inline(always)]
pub fn hash_to_parent(node: &mut Node, sibling: &Node, is_left: bool) {
    let parent = if is_left {
        hashv(&[node, sibling])
    } else {
        hashv(&[sibling, node])
    };
    node.copy_from_slice(parent.as_ref())
}

/// Fills in proof to the height of the concurrent merkle tree.
/// Missing nodes are inferred as empty node hashes.
pub fn fill_in_proof<const MAX_DEPTH: usize>(
    proof_vec: &[Node],
    full_proof: &mut [Node; MAX_DEPTH],
) {
    solana_logging!("Attempting to fill in proof");
    if !proof_vec.is_empty() {
        full_proof[..proof_vec.len()].copy_from_slice(proof_vec);
    }

    for (i, item) in full_proof
        .iter_mut()
        .enumerate()
        .take(MAX_DEPTH)
        .skip(proof_vec.len())
    {
        *item = empty_node(i as u32);
    }
}
