#![allow(clippy::arithmetic_side_effects)]
use {
    rand::{self, thread_rng, Rng},
    spl_concurrent_merkle_tree::{
        concurrent_merkle_tree::{
            ConcurrentMerkleTree, FillEmptyOrAppendArgs, InitializeWithRootArgs, ProveLeafArgs,
            SetLeafArgs,
        },
        error::ConcurrentMerkleTreeError,
        node::{Node, EMPTY},
    },
    spl_merkle_tree_reference::MerkleTree,
};

const DEPTH: usize = 10;
const BUFFER_SIZE: usize = 64;

fn setup() -> (ConcurrentMerkleTree<DEPTH, BUFFER_SIZE>, MerkleTree) {
    // On-chain CMT
    let cmt = ConcurrentMerkleTree::<DEPTH, BUFFER_SIZE>::new();
    // Init off-chain Merkle tree with corresponding # of leaves
    let leaves = vec![EMPTY; 1 << DEPTH];
    // Off-chain merkle tree
    let reference_tree = MerkleTree::new(leaves.as_slice());

    (cmt, reference_tree)
}

#[tokio::test(flavor = "multi_thread")]
async fn test_initialize() {
    let (mut cmt, off_chain_tree) = setup();
    cmt.initialize().unwrap();

    assert_eq!(
        cmt.get_change_log().root,
        off_chain_tree.get_root(),
        "Init failed to set root properly"
    );

    // Check that reinitialization fails
    if let Err(ConcurrentMerkleTreeError::TreeAlreadyInitialized) = cmt.initialize() {
        println!("Reinitialization successfully prevented");
    } else {
        panic!("Tree should not be able to be reinitialized");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_bypass_initialize() {
    let (mut cmt, off_chain_tree) = setup();
    let mut rng = thread_rng();
    let leaf = rng.gen::<[u8; 32]>();

    assert_eq!(
        ConcurrentMerkleTreeError::TreeNotInitialized,
        cmt.append(leaf).unwrap_err(),
        "Expected TreeNotInitialized error when appending to uninitialized tree"
    );

    assert_eq!(
        ConcurrentMerkleTreeError::TreeNotInitialized,
        cmt.set_leaf(&SetLeafArgs {
            current_root: off_chain_tree.get_root(),
            previous_leaf: [0; 32],
            new_leaf: leaf,
            proof_vec: off_chain_tree.get_proof_of_leaf(0),
            index: 0
        },)
            .unwrap_err(),
        "Expected TreeNotInitialized error when setting a leaf on an uninitialized tree"
    );

    assert_eq!(
        ConcurrentMerkleTreeError::TreeNotInitialized,
        cmt.prove_leaf(&ProveLeafArgs {
            current_root: off_chain_tree.get_root(),
            leaf,
            proof_vec: off_chain_tree.get_proof_of_leaf(0),
            index: 0
        })
        .unwrap_err(),
        "Expected TreeNotInitialized error when proving a leaf exists on an uninitialized tree"
    );

    assert_eq!(
        ConcurrentMerkleTreeError::TreeNotInitialized,
        cmt.fill_empty_or_append(
            &FillEmptyOrAppendArgs { current_root: off_chain_tree.get_root(), leaf, proof_vec: off_chain_tree.get_proof_of_leaf(0), index: 0 }
        )
        .unwrap_err(),
        "Expected TreeNotInitialized error when filling an empty leaf or appending to uninitialized tree"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_append() {
    let (mut cmt, mut off_chain_tree) = setup();
    let mut rng = thread_rng();
    cmt.initialize().unwrap();

    for i in 0..(1 << DEPTH) {
        let leaf = rng.gen::<[u8; 32]>();
        cmt.append(leaf).unwrap();
        off_chain_tree.add_leaf(leaf, i);
        assert_eq!(
            cmt.get_change_log().root,
            off_chain_tree.get_root(),
            "On chain tree failed to update properly on an append",
        );
    }

    assert_eq!(
        cmt.buffer_size, BUFFER_SIZE as u64,
        "CMT buffer size is wrong"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_prove_leaf() {
    let (mut cmt, mut off_chain_tree) = setup();
    let mut rng = thread_rng();
    cmt.initialize().unwrap();

    for i in 0..(1 << DEPTH) {
        let leaf = rng.gen::<[u8; 32]>();
        cmt.append(leaf).unwrap();
        off_chain_tree.add_leaf(leaf, i);
    }

    // Test that all leaves can be verified
    for leaf_index in 0..(1 << DEPTH) {
        cmt.prove_leaf(&ProveLeafArgs {
            current_root: off_chain_tree.get_root(),
            leaf: off_chain_tree.get_leaf(leaf_index),
            proof_vec: off_chain_tree.get_proof_of_leaf(leaf_index),
            index: leaf_index as u32,
        })
        .unwrap();
    }

    // Test that old proofs can be verified
    // Up to BUFFER_SIZE old
    let num_leaves_to_try = 10;
    for _ in 0..num_leaves_to_try {
        let leaf_idx = rng.gen_range(0..1 << DEPTH);
        let _last_leaf_idx = off_chain_tree.leaf_nodes.len() - 1;
        let root = off_chain_tree.get_root();
        let leaf = off_chain_tree.get_leaf(leaf_idx);
        let old_proof = off_chain_tree.get_proof_of_leaf(leaf_idx);

        // While executing random replaces, check
        for _ in 0..(BUFFER_SIZE - 1) {
            let new_leaf = rng.gen::<Node>();
            let mut random_leaf_idx = rng.gen_range(0..1 << DEPTH);
            while random_leaf_idx == leaf_idx {
                random_leaf_idx = rng.gen_range(0..1 << DEPTH);
            }

            cmt.set_leaf(&SetLeafArgs {
                current_root: off_chain_tree.get_root(),
                previous_leaf: off_chain_tree.get_leaf(random_leaf_idx),
                new_leaf,
                proof_vec: off_chain_tree.get_proof_of_leaf(random_leaf_idx),
                index: random_leaf_idx as u32,
            })
            .unwrap();
            off_chain_tree.add_leaf(new_leaf, random_leaf_idx);

            // Assert that we can still prove existence of our mostly unused leaf
            cmt.prove_leaf(&ProveLeafArgs {
                current_root: root,
                leaf,
                proof_vec: old_proof.clone(),
                index: leaf_idx as u32,
            })
            .unwrap();
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_initialize_with_root() {
    let (mut cmt, mut tree) = setup();
    let mut rng = thread_rng();

    for i in 0..(1 << DEPTH) {
        tree.add_leaf(rng.gen::<[u8; 32]>(), i);
    }

    let last_leaf_idx = tree.leaf_nodes.len() - 1;
    cmt.initialize_with_root(&InitializeWithRootArgs {
        root: tree.get_root(),
        rightmost_leaf: tree.get_leaf(last_leaf_idx),
        proof_vec: tree.get_proof_of_leaf(last_leaf_idx),
        index: last_leaf_idx as u32,
    })
    .unwrap();

    assert_eq!(
        cmt.get_change_log().root,
        tree.get_root(),
        "Init failed to set root properly"
    );

    // Check that reinitialization fails
    if let Err(ConcurrentMerkleTreeError::TreeAlreadyInitialized) =
        cmt.initialize_with_root(&InitializeWithRootArgs {
            root: tree.get_root(),
            rightmost_leaf: tree.get_leaf(last_leaf_idx),
            proof_vec: tree.get_proof_of_leaf(last_leaf_idx),
            index: last_leaf_idx as u32,
        })
    {
        println!("Reinitialization with root successfully prevented");
    } else {
        panic!("Tree should not be able to be reinitialized");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_leaf_contents_modified() {
    let (mut cmt, mut tree) = setup();
    let mut rng = thread_rng();
    cmt.initialize().unwrap();

    // Create tree with a single leaf
    let leaf = rng.gen::<[u8; 32]>();
    tree.add_leaf(leaf, 0);
    cmt.append(leaf).unwrap();

    // Save a proof of this leaf
    let root = tree.get_root();
    let proof = tree.get_proof_of_leaf(0);

    // Update leaf to be something else
    let new_leaf_0 = rng.gen::<[u8; 32]>();
    tree.add_leaf(leaf, 0);
    cmt.set_leaf(&SetLeafArgs {
        current_root: root,
        previous_leaf: leaf,
        new_leaf: new_leaf_0,
        proof_vec: proof.clone(),
        index: 0_u32,
    })
    .unwrap();

    // Should fail to replace same leaf using outdated info
    let new_leaf_1 = rng.gen::<[u8; 32]>();
    tree.add_leaf(leaf, 0);
    match cmt.set_leaf(&SetLeafArgs {
        current_root: root,
        previous_leaf: leaf,
        new_leaf: new_leaf_1,
        proof_vec: proof,
        index: 0u32,
    }) {
        Ok(_) => {
            panic!("CMT should fail when replacing leafs with outdated leaf proofs")
        }
        Err(e) => match e {
            ConcurrentMerkleTreeError::LeafContentsModified => {}
            _ => {
                panic!("Wrong error was thrown: {:?}", e);
            }
        },
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_replaces() {
    let (mut cmt, mut tree) = setup();
    let mut rng = thread_rng();
    cmt.initialize().unwrap();

    // Fill both trees with random nodes
    for i in 0..(1 << DEPTH) {
        let leaf = rng.gen::<[u8; 32]>();
        tree.add_leaf(leaf, i);
        cmt.append(leaf).unwrap();
    }
    assert_eq!(cmt.get_change_log().root, tree.get_root());

    // Replace leaves in order
    for i in 0..(1 << DEPTH) {
        let leaf = rng.gen::<[u8; 32]>();
        cmt.set_leaf(&SetLeafArgs {
            current_root: tree.get_root(),
            previous_leaf: tree.get_leaf(i),
            new_leaf: leaf,
            proof_vec: tree.get_proof_of_leaf(i),
            index: i as u32,
        })
        .unwrap();
        tree.add_leaf(leaf, i);
        assert_eq!(cmt.get_change_log().root, tree.get_root());
    }

    // Replaces leaves in a random order by x capacity
    let test_capacity: usize = 1 << (DEPTH - 1);
    for _ in 0..(test_capacity) {
        let index = rng.gen_range(0..test_capacity) % (1 << DEPTH);
        let leaf = rng.gen::<[u8; 32]>();
        cmt.set_leaf(&SetLeafArgs {
            current_root: tree.get_root(),
            previous_leaf: tree.get_leaf(index),
            new_leaf: leaf,
            proof_vec: tree.get_proof_of_leaf(index),
            index: index as u32,
        })
        .unwrap();
        tree.add_leaf(leaf, index);
        assert_eq!(cmt.get_change_log().root, tree.get_root());
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_default_node_is_empty() {
    assert_eq!(
        Node::default(),
        EMPTY,
        "Expected default() to be the empty node"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_mixed() {
    let (mut cmt, mut tree) = setup();
    let mut rng = thread_rng();
    cmt.initialize().unwrap();

    // Fill both trees with random nodes
    let mut tree_size = 10;
    for i in 0..tree_size {
        let leaf = rng.gen::<[u8; 32]>();
        tree.add_leaf(leaf, i);
        cmt.append(leaf).unwrap();
    }
    assert_eq!(cmt.get_change_log().root, tree.get_root());

    // Replaces leaves in a random order by 4x capacity
    let mut last_rmp = cmt.rightmost_proof;

    let tree_capacity: usize = 1 << DEPTH;
    while tree_size < tree_capacity {
        let leaf = rng.gen::<[u8; 32]>();
        let random_num: u32 = rng.gen_range(0..10);
        if random_num < 5 {
            println!("{} append", tree_size);
            cmt.append(leaf).unwrap();
            tree.add_leaf(leaf, tree_size);
            tree_size += 1;
        } else {
            let index = rng.gen_range(0..tree_size) % (tree_size);
            println!("{} replace {}", tree_size, index);
            cmt.set_leaf(&SetLeafArgs {
                current_root: tree.get_root(),
                previous_leaf: tree.get_leaf(index),
                new_leaf: leaf,
                proof_vec: tree.get_proof_of_leaf(index),
                index: index as u32,
            })
            .unwrap();
            tree.add_leaf(leaf, index);
        }
        if cmt.get_change_log().root != tree.get_root() {
            let last_active_index: usize =
                (cmt.active_index as usize + BUFFER_SIZE - 1) % BUFFER_SIZE;
            println!("{:?}", &last_rmp);
            println!("{:?}", &cmt.change_logs[last_active_index]);
            println!("{:?}", &cmt.get_change_log())
        }
        last_rmp = cmt.rightmost_proof;
        assert_eq!(cmt.get_change_log().root, tree.get_root());
    }
}

#[tokio::test(flavor = "multi_thread")]
/// Append after replacing the last leaf
async fn test_append_bug_repro_1() {
    let (mut cmt, mut tree) = setup();
    let mut rng = thread_rng();
    cmt.initialize().unwrap();

    // Fill both trees with random nodes
    let tree_size = 10;
    for i in 0..tree_size {
        let leaf = rng.gen::<[u8; 32]>();
        tree.add_leaf(leaf, i);
        cmt.append(leaf).unwrap();
    }
    assert_eq!(cmt.get_change_log().root, tree.get_root());

    // Replace the rightmost leaf
    let leaf_0 = rng.gen::<[u8; 32]>();
    let index = 9;
    cmt.set_leaf(&SetLeafArgs {
        current_root: tree.get_root(),
        previous_leaf: tree.get_leaf(index),
        new_leaf: leaf_0,
        proof_vec: tree.get_proof_of_leaf(index),
        index: index as u32,
    })
    .unwrap();
    tree.add_leaf(leaf_0, index);

    let last_rmp = cmt.rightmost_proof;

    // Append
    let leaf_1 = rng.gen::<[u8; 32]>();
    cmt.append(leaf_1).unwrap();
    tree.add_leaf(leaf_1, tree_size);

    // Now compare something
    if cmt.get_change_log().root != tree.get_root() {
        let _last_active_index: usize = (cmt.active_index as usize + BUFFER_SIZE - 1) % BUFFER_SIZE;
        println!("{:?}", &last_rmp);
    }
    assert_eq!(cmt.get_change_log().root, tree.get_root());
}

#[tokio::test(flavor = "multi_thread")]
/// Append after also appending via a replace
async fn test_append_bug_repro_2() {
    let (mut cmt, mut tree) = setup();
    let mut rng = thread_rng();
    cmt.initialize().unwrap();

    // Fill both trees with random nodes
    let mut tree_size = 10;
    for i in 0..tree_size {
        let leaf = rng.gen::<[u8; 32]>();
        tree.add_leaf(leaf, i);
        cmt.append(leaf).unwrap();
    }
    assert_eq!(cmt.get_change_log().root, tree.get_root());

    // Replace the rightmost leaf
    let mut leaf = rng.gen::<[u8; 32]>();
    let index = 10;
    cmt.set_leaf(&SetLeafArgs {
        current_root: tree.get_root(),
        previous_leaf: tree.get_leaf(index),
        new_leaf: leaf,
        proof_vec: tree.get_proof_of_leaf(index),
        index: index as u32,
    })
    .unwrap();
    tree.add_leaf(leaf, index);
    tree_size += 1;

    let last_rmp = cmt.rightmost_proof;

    // Append
    leaf = rng.gen::<[u8; 32]>();
    cmt.append(leaf).unwrap();
    tree.add_leaf(leaf, tree_size);

    // Now compare something
    if cmt.get_change_log().root != tree.get_root() {
        let _last_active_index: usize = (cmt.active_index as usize + BUFFER_SIZE - 1) % BUFFER_SIZE;
        println!("{:?}", &last_rmp);
    }
    assert_eq!(cmt.get_change_log().root, tree.get_root());
}

#[tokio::test(flavor = "multi_thread")]
/// Test that empty trees are checked properly by adding & removing leaves one
/// by one
async fn test_prove_tree_empty_incremental() {
    let (mut cmt, mut tree) = setup();
    let mut rng = thread_rng();
    cmt.initialize().unwrap();

    cmt.prove_tree_is_empty().unwrap();

    // Append a random leaf & remove it, and make sure that the tree is empty at the
    // end
    let tree_size = 64;
    for i in 0..tree_size {
        let leaf = rng.gen::<[u8; 32]>();
        cmt.append(leaf).unwrap();
        tree.add_leaf(leaf, i);

        match cmt.prove_tree_is_empty() {
            Ok(_) => {
                panic!("Tree has a leaf in it -- should not be possible to prove empty!")
            }
            Err(e) => match e {
                ConcurrentMerkleTreeError::TreeNonEmpty => {}
                _ => {
                    panic!("Wrong error thrown. Expected TreeNonEmpty erro")
                }
            },
        }

        cmt.set_leaf(&SetLeafArgs {
            current_root: tree.get_root(),
            previous_leaf: tree.get_leaf(i),
            new_leaf: EMPTY,
            proof_vec: tree.get_proof_of_leaf(i),
            index: i as u32,
        })
        .unwrap();
        tree.add_leaf(EMPTY, i);

        cmt.prove_tree_is_empty().unwrap();
    }
}

#[tokio::test(flavor = "multi_thread")]
/// Test that empty trees are checked properly by adding & removing leaves in a
/// batch
async fn test_prove_tree_empty_batched() {
    let (mut cmt, mut tree) = setup();
    let mut rng = thread_rng();
    cmt.initialize().unwrap();

    // Sanity check
    cmt.prove_tree_is_empty().unwrap();

    // Add random leaves to the tree
    let tree_size = 64;
    for i in 0..tree_size {
        let leaf = rng.gen::<[u8; 32]>();
        cmt.append(leaf).unwrap();
        tree.add_leaf(leaf, i);

        match cmt.prove_tree_is_empty() {
            Ok(_) => {
                panic!("Tree has a leaf in it -- should not be possible to prove empty!")
            }
            Err(e) => match e {
                ConcurrentMerkleTreeError::TreeNonEmpty => {}
                _ => {
                    panic!("Wrong error thrown. Expected TreeNonEmpty erro")
                }
            },
        }
    }
    // Remove random leaves
    for i in 0..tree_size - 1 {
        cmt.set_leaf(&SetLeafArgs {
            current_root: tree.get_root(),
            previous_leaf: tree.get_leaf(i),
            new_leaf: EMPTY,
            proof_vec: tree.get_proof_of_leaf(i),
            index: i as u32,
        })
        .unwrap();
        tree.add_leaf(EMPTY, i);

        match cmt.prove_tree_is_empty() {
            Ok(_) => {
                panic!("Tree has a leaf in it -- should not be possible to prove empty!")
            }
            Err(e) => match e {
                ConcurrentMerkleTreeError::TreeNonEmpty => {}
                _ => {
                    panic!("Wrong error thrown. Expected TreeNonEmpty erro")
                }
            },
        }
    }
    cmt.set_leaf(&SetLeafArgs {
        current_root: tree.get_root(),
        previous_leaf: tree.get_leaf(tree_size - 1),
        new_leaf: EMPTY,
        proof_vec: tree.get_proof_of_leaf(tree_size - 1),
        index: (tree_size - 1) as u32,
    })
    .unwrap();
    tree.add_leaf(EMPTY, tree_size - 1);

    // Check that the last leaf was successfully removed
    cmt.prove_tree_is_empty().unwrap();
}
