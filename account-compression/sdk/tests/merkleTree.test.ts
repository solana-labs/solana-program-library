import { strict as assert } from 'node:assert';

import * as crypto from 'crypto';

import { emptyNode, MerkleTree } from '../src';

describe('MerkleTree tests', () => {
    it('Check constructor equivalence for depth 2 tree', () => {
        const leaves = [crypto.randomBytes(32), crypto.randomBytes(32), crypto.randomBytes(32)];
        const rawLeaves = leaves.concat(emptyNode(0));
        const merkleTreeRaw = new MerkleTree(rawLeaves);
        const merkleTreeSparse = MerkleTree.sparseMerkleTreeFromLeaves(leaves, 2);

        assert(merkleTreeRaw.root.equals(merkleTreeSparse.root));
    });

    const TEST_DEPTH = 14;
    it(`Check proofs for 2^${TEST_DEPTH} tree`, () => {
        const leaves: Buffer[] = [];
        for (let i = 0; i < 2 ** TEST_DEPTH; i++) {
            leaves.push(crypto.randomBytes(32));
        }
        const merkleTree = new MerkleTree(leaves);

        // Check proofs
        for (let i = 0; i < leaves.length; i++) {
            const proof = merkleTree.getProof(i);
            assert(MerkleTree.verify(merkleTree.getRoot(), proof));
        }
    });
});
