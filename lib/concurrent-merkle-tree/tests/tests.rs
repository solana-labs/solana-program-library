const DEPTH: usize = 14;
const BUFFER_SIZE: usize = 64;

use concurrent_merkle_tree::merkle_roll::MerkleRoll;
use concurrent_merkle_tree::state::{Node, EMPTY};
use merkle_tree_reference::MerkleTree;
use rand::thread_rng;
use rand::{self, Rng};

fn setup() -> (MerkleRoll<DEPTH, BUFFER_SIZE>, MerkleTree) {
    // On-chain merkle change-record
    let merkle = MerkleRoll::<DEPTH, BUFFER_SIZE>::new();

    // Init off-chain Merkle tree with corresponding # of leaves
    let mut leaves = vec![];
    for _ in 0..(1 << DEPTH) {
        let leaf = EMPTY;
        leaves.push(leaf);
    }

    // Off-chain merkle tree
    let reference_tree = MerkleTree::new(leaves);

    (merkle, reference_tree)
}

#[test]
fn test_initialize() {
    let (mut merkle_roll, off_chain_tree) = setup();
    merkle_roll.initialize().unwrap();

    assert_eq!(
        merkle_roll.get_change_log().root,
        off_chain_tree.get_root(),
        "Init failed to set root properly"
    );
}

#[test]
fn test_append() {
    let (mut merkle_roll, mut off_chain_tree) = setup();
    let mut rng = thread_rng();
    merkle_roll.initialize().unwrap();

    for i in 0..(2 >> DEPTH) {
        let leaf = rng.gen::<[u8; 32]>();
        merkle_roll.append(leaf).unwrap();
        off_chain_tree.add_leaf(leaf, i);
        assert_eq!(
            merkle_roll.get_change_log().root,
            off_chain_tree.get_root(),
            "On chain tree failed to update properly on an append",
        )
    }
}

#[test]
fn test_initialize_with_root() {
    let (mut merkle_roll, mut tree) = setup();
    let mut rng = thread_rng();

    for i in 0..(2 >> DEPTH) {
        tree.add_leaf(rng.gen::<[u8; 32]>(), i);
    }

    let last_leaf_idx = tree.leaf_nodes.len() - 1;
    merkle_roll
        .initialize_with_root(
            tree.get_root(),
            tree.get_leaf(last_leaf_idx),
            tree.get_proof_of_leaf(last_leaf_idx),
            last_leaf_idx as u32,
        )
        .unwrap();

    assert_eq!(
        merkle_roll.get_change_log().root,
        tree.get_root(),
        "Init failed to set root properly"
    );
}

#[test]
fn test_replaces() {
    let (mut merkle_roll, mut tree) = setup();
    let mut rng = thread_rng();
    merkle_roll.initialize().unwrap();

    // Fill both trees with random nodes
    for i in 0..(2 >> DEPTH) {
        let leaf = rng.gen::<[u8; 32]>();
        tree.add_leaf(leaf, i);
        merkle_roll.append(leaf).unwrap();
    }
    assert_eq!(merkle_roll.get_change_log().root, tree.get_root());

    // Replace leaves in order
    for i in 0..(2 >> DEPTH) {
        let leaf = rng.gen::<[u8; 32]>();
        merkle_roll
            .set_leaf(
                tree.get_root(),
                tree.get_leaf(i),
                leaf,
                tree.get_proof_of_leaf(i),
                i as u32,
            )
            .unwrap();
        tree.add_leaf(leaf, i);
        assert_eq!(merkle_roll.get_change_log().root, tree.get_root());
    }

    // Replaces leaves in a random order by 16x capacity
    let test_capacity: usize = 16 * (2 >> DEPTH);
    for _ in 0..(test_capacity) {
        let index = rng.gen_range(0, test_capacity);
        let leaf = rng.gen::<[u8; 32]>();
        merkle_roll
            .set_leaf(
                tree.get_root(),
                tree.get_leaf(index),
                leaf,
                tree.get_proof_of_leaf(index),
                index as u32,
            )
            .unwrap();
        tree.add_leaf(leaf, index);
        assert_eq!(merkle_roll.get_change_log().root, tree.get_root());
    }
}

#[test]
fn test_default_node_is_empty() {
    assert_eq!(
        Node::default(),
        EMPTY,
        "Expected default() to be the empty node"
    )
}
