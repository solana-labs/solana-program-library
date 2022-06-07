use concurrent_merkle_tree::error::CMTError;
use concurrent_merkle_tree::merkle_roll::MerkleRoll;
use concurrent_merkle_tree::state::{Node, EMPTY};
use merkle_tree_reference::MerkleTree;
use rand::thread_rng;
use rand::{self, Rng};
use tokio;

const DEPTH: usize = 14;
const BUFFER_SIZE: usize = 64;

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

#[tokio::test(threaded_scheduler)]
async fn test_initialize() {
    let (mut merkle_roll, off_chain_tree) = setup();
    merkle_roll.initialize().unwrap();

    assert_eq!(
        merkle_roll.get_change_log().root,
        off_chain_tree.get_root(),
        "Init failed to set root properly"
    );
}

#[tokio::test(threaded_scheduler)]
async fn test_append() {
    let (mut merkle_roll, mut off_chain_tree) = setup();
    let mut rng = thread_rng();
    merkle_roll.initialize().unwrap();

    for i in 0..(1 << DEPTH) {
        let leaf = rng.gen::<[u8; 32]>();
        merkle_roll.append(leaf).unwrap();
        off_chain_tree.add_leaf(leaf, i);
        assert_eq!(
            merkle_roll.get_change_log().root,
            off_chain_tree.get_root(),
            "On chain tree failed to update properly on an append",
        );
    }

    assert_eq!(
        merkle_roll.buffer_size, BUFFER_SIZE as u64,
        "Merkle roll buffer size is wrong"
    );
}

#[tokio::test(threaded_scheduler)]
async fn test_prove_leaf() {
    let (mut merkle_roll, mut off_chain_tree) = setup();
    let mut rng = thread_rng();
    merkle_roll.initialize().unwrap();

    for i in 0..(1 << DEPTH) {
        let leaf = rng.gen::<[u8; 32]>();
        merkle_roll.append(leaf).unwrap();
        off_chain_tree.add_leaf(leaf, i);
    }

    // Test that all leaves can be verified
    for leaf_index in 0..(1 << DEPTH) {
        merkle_roll
            .prove_leaf(
                off_chain_tree.get_root(),
                off_chain_tree.get_leaf(leaf_index),
                &off_chain_tree.get_proof_of_leaf(leaf_index),
                leaf_index as u32,
            )
            .unwrap();
    }

    // Test that old proofs can be verified
    // Up to BUFFER_SIZE old
    let num_leaves_to_try = 10;
    for _ in 0..num_leaves_to_try {
        let leaf_idx = rng.gen_range(0, 1 << DEPTH);
        let last_leaf_idx = off_chain_tree.leaf_nodes.len() - 1;
        let root = off_chain_tree.get_root();
        let leaf = off_chain_tree.get_leaf(leaf_idx);
        let old_proof = off_chain_tree.get_proof_of_leaf(leaf_idx);

        // While executing random replaces, check
        for _ in 0..BUFFER_SIZE {
            let new_leaf = rng.gen::<Node>();
            let mut random_leaf_idx = rng.gen_range(0, 1 << DEPTH);
            while random_leaf_idx == leaf_idx {
                random_leaf_idx = rng.gen_range(0, 1 << DEPTH);
            }

            merkle_roll
                .set_leaf(
                    off_chain_tree.get_root(),
                    off_chain_tree.get_leaf(random_leaf_idx),
                    new_leaf,
                    &off_chain_tree.get_proof_of_leaf(random_leaf_idx),
                    random_leaf_idx as u32,
                )
                .unwrap();
            off_chain_tree.add_leaf(new_leaf, random_leaf_idx);

            // Assert that we can still prove existence of our mostly unused leaf
            merkle_roll
                .prove_leaf(root, leaf, &old_proof, leaf_idx as u32)
                .unwrap();
        }
    }
}

#[tokio::test(threaded_scheduler)]
async fn test_initialize_with_root() {
    let (mut merkle_roll, mut tree) = setup();
    let mut rng = thread_rng();

    for i in 0..(1 << DEPTH) {
        tree.add_leaf(rng.gen::<[u8; 32]>(), i);
    }

    let last_leaf_idx = tree.leaf_nodes.len() - 1;
    merkle_roll
        .initialize_with_root(
            tree.get_root(),
            tree.get_leaf(last_leaf_idx),
            &tree.get_proof_of_leaf(last_leaf_idx),
            last_leaf_idx as u32,
        )
        .unwrap();

    assert_eq!(
        merkle_roll.get_change_log().root,
        tree.get_root(),
        "Init failed to set root properly"
    );
}

#[tokio::test(threaded_scheduler)]
async fn test_leaf_contents_modified() {
    let (mut merkle_roll, mut tree) = setup();
    let mut rng = thread_rng();
    merkle_roll.initialize().unwrap();

    // Create tree with a single leaf
    let leaf = rng.gen::<[u8; 32]>();
    tree.add_leaf(leaf, 0);
    merkle_roll.append(leaf).unwrap();

    // Save a proof of this leaf
    let root = tree.get_root();
    let proof = tree.get_proof_of_leaf(0);

    // Update leaf to be something else
    let new_leaf_0 = rng.gen::<[u8; 32]>();
    tree.add_leaf(leaf, 0);
    merkle_roll
        .set_leaf(root, leaf, new_leaf_0, &proof, 0 as u32)
        .unwrap();

    // Should fail to replace same leaf using outdated info
    let new_leaf_1 = rng.gen::<[u8; 32]>();
    tree.add_leaf(leaf, 0);
    match merkle_roll.set_leaf(root, leaf, new_leaf_1, &proof, 0 as u32) {
        Ok(_) => {
            assert!(
                false,
                "Merkle roll should fail when replacing leafs with outdated leaf proofs"
            )
        }
        Err(e) => match e {
            CMTError::LeafContentsModified => {}
            _ => {
                // println!()
                assert!(false, "Wrong error was thrown: {:?}", e);
            }
        },
    }
}

#[tokio::test(threaded_scheduler)]
async fn test_replaces() {
    let (mut merkle_roll, mut tree) = setup();
    let mut rng = thread_rng();
    merkle_roll.initialize().unwrap();

    // Fill both trees with random nodes
    for i in 0..(1 << DEPTH) {
        let leaf = rng.gen::<[u8; 32]>();
        tree.add_leaf(leaf, i);
        merkle_roll.append(leaf).unwrap();
    }
    assert_eq!(merkle_roll.get_change_log().root, tree.get_root());

    // Replace leaves in order
    for i in 0..(1 << DEPTH) {
        let leaf = rng.gen::<[u8; 32]>();
        merkle_roll
            .set_leaf(
                tree.get_root(),
                tree.get_leaf(i),
                leaf,
                &tree.get_proof_of_leaf(i),
                i as u32,
            )
            .unwrap();
        tree.add_leaf(leaf, i);
        assert_eq!(merkle_roll.get_change_log().root, tree.get_root());
    }

    // Replaces leaves in a random order by 4x capacity
    let test_capacity: usize = 4 * (1 << DEPTH);
    for _ in 0..(test_capacity) {
        let index = rng.gen_range(0, test_capacity) % (1 << DEPTH);
        let leaf = rng.gen::<[u8; 32]>();
        merkle_roll
            .set_leaf(
                tree.get_root(),
                tree.get_leaf(index),
                leaf,
                &tree.get_proof_of_leaf(index),
                index as u32,
            )
            .unwrap();
        tree.add_leaf(leaf, index);
        assert_eq!(merkle_roll.get_change_log().root, tree.get_root());
    }
}

#[tokio::test(threaded_scheduler)]
async fn test_default_node_is_empty() {
    assert_eq!(
        Node::default(),
        EMPTY,
        "Expected default() to be the empty node"
    );
}
