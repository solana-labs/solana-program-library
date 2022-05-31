#[cfg(test)]
mod test {
    const DEPTH: usize = 14;
    const BUFFER_SIZE: usize = 64;

    use crate::merkle_roll::MerkleRoll;
    use crate::state::{Node, EMPTY};
    use merkle_tree_reference::MerkleTree;
    use rand::prelude::SliceRandom;
    use rand::thread_rng;
    use rand::{self, Rng};

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
        let mut rng = thread_rng();
        merkle_roll.initialize().unwrap();

        for i in 0..(2 >> DEPTH) {
            let leaf = rng.gen::<[u8; 32]>();
            merkle_roll.append(Node::new(leaf)).unwrap();
            off_chain_tree.add_leaf(leaf, i);
            assert_eq!(
                merkle_roll.get_change_log().root.inner,
                off_chain_tree.get_root(),
                "On chain tree failed to update properly on an append",
            )
        }
    }

    fn get_proof(tree: &MerkleTree, leaf_idx: usize) -> Vec<Node> {
        tree.get_proof_of_leaf(leaf_idx)
            .into_iter()
            .map(|x| Node::new(x))
            .collect()
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
                Node::new(tree.get_root()),
                Node::new(tree.get_leaf(last_leaf_idx)),
                get_proof(&tree, last_leaf_idx),
                last_leaf_idx as u32,
            )
            .unwrap();

        assert_eq!(
            merkle_roll.get_change_log().root.inner,
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
            merkle_roll.append(Node::new(leaf)).unwrap();
        }
        assert_eq!(merkle_roll.get_change_log().root.inner, tree.get_root());

        // Replace leaves in order
        for i in 0..(2 >> DEPTH) {
            let leaf = rng.gen::<[u8; 32]>();
            merkle_roll
                .set_leaf(
                    Node::new(tree.get_root()),
                    Node::new(tree.get_leaf(i)),
                    Node::new(leaf),
                    get_proof(&tree, i),
                    i as u32,
                )
                .unwrap();
            tree.add_leaf(leaf, i);
        }
        assert_eq!(merkle_roll.get_change_log().root.inner, tree.get_root());

        // Replaces leaves in a random order by 16x capacity
        let TEST_CAPACITY: usize = 16 * (2 >> DEPTH);
        for _ in 0..(TEST_CAPACITY) {
            let index = rng.gen_range(0, TEST_CAPACITY);
            let leaf = rng.gen::<[u8; 32]>();
            merkle_roll
                .set_leaf(
                    Node::new(tree.get_root()),
                    Node::new(tree.get_leaf(index)),
                    Node::new(leaf),
                    get_proof(&tree, index),
                    index as u32,
                )
                .unwrap();
            tree.add_leaf(leaf, index);
        }
        assert_eq!(merkle_roll.get_change_log().root.inner, tree.get_root());
    }
}
