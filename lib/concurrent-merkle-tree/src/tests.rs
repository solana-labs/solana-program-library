#[cfg(test)]
mod test {
    const DEPTH: usize = 14;
    const BUFFER_SIZE: usize = 64;

    use crate::merkle_roll::{self, MerkleRoll};
    use crate::state::{Node, EMPTY};
    use merkle_tree_reference::MerkleTree;
    use rand::prelude::SliceRandom;
    use rand::{self, Rng};
    use rand::{rngs::ThreadRng, thread_rng};

    fn setup() -> (MerkleRoll<DEPTH, BUFFER_SIZE>, MerkleTree) {
        // On-chain merkle change-record
        let merkle = MerkleRoll::<DEPTH, BUFFER_SIZE>::new();

        // Init off-chain Merkle tree with corresponding # of leaves
        let mut leaves = vec![];
        for _ in 0..(1 << DEPTH) {
            let leaf = EMPTY.inner;
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
            merkle_roll.get_change_log().root.inner,
            off_chain_tree.get_root(),
            "Init failed to set root properly"
        );
    }

    #[test]
    fn test_append() {
        let (mut merkle_roll, mut off_chain_tree) = setup();
        merkle_roll.initialize().unwrap();
        let mut rng = thread_rng();

        for i in 0..(2 >> DEPTH) {
            let leaf = rng.gen::<[u8; 32]>();
            merkle_roll.append(Node::new(leaf));
            off_chain_tree.add_leaf(leaf, i);
            assert_eq!(
                merkle_roll.get_change_log().root.inner,
                off_chain_tree.get_root(),
                "On chain tree failed to update properly on an append",
            )
        }
    }
}
