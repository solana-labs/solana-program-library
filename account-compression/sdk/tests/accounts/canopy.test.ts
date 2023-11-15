import { AnchorProvider } from '@project-serum/anchor';
import NodeWallet from '@project-serum/anchor/dist/cjs/nodewallet';
import { Connection, Keypair, PublicKey, TransactionInstruction } from '@solana/web3.js';
import { assert } from 'chai';
import * as crypto from 'crypto';

import {
    ConcurrentMerkleTreeAccount,
    createAppendIx,
    createReplaceIx,
    createVerifyLeafIx,
    ValidDepthSizePair,
} from '../../src';
import { MerkleTree } from '../../src/merkle-tree';
import { createTreeOnChain, execute } from '../utils';

describe(`Canopy test`, () => {
    let offChainTree: MerkleTree;
    let cmtKeypair: Keypair;
    let cmt: PublicKey;
    let payerKeypair: Keypair;
    let payer: PublicKey;
    let connection: Connection;
    let provider: AnchorProvider;

    const MAX_BUFFER_SIZE = 8;
    const MAX_DEPTH = 5;
    const DEPTH_SIZE_PAIR: ValidDepthSizePair = {
        maxBufferSize: MAX_BUFFER_SIZE,
        maxDepth: MAX_DEPTH,
    };

    beforeEach(async () => {
        payerKeypair = Keypair.generate();
        payer = payerKeypair.publicKey;
        connection = new Connection('http://127.0.0.1:8899', {
            commitment: 'confirmed',
        });
        const wallet = new NodeWallet(payerKeypair);
        provider = new AnchorProvider(connection, wallet, {
            commitment: connection.commitment,
            skipPreflight: true,
        });

        await provider.connection.confirmTransaction(
            await provider.connection.requestAirdrop(payer, 1e10),
            'confirmed'
        );
    });

    describe(`Unit test proof instructions`, () => {
        beforeEach(async () => {
            [cmtKeypair, offChainTree] = await createTreeOnChain(
                provider,
                payerKeypair,
                2 ** MAX_DEPTH, // Fill up the tree
                DEPTH_SIZE_PAIR,
                MAX_DEPTH // Store full tree on chain
            );
            cmt = cmtKeypair.publicKey;
        });
        it(`VerifyLeaf works with no proof accounts`, async () => {
            const splCMT = await ConcurrentMerkleTreeAccount.fromAccountAddress(connection, cmt, 'confirmed');

            // Test that the entire state of the tree is stored properly
            // by verifying every leaf in the tree.
            // We use batches of 4 verify ixs / tx to speed up the test
            let i = 0;
            const stepSize = 4;
            while (i < 2 ** MAX_DEPTH) {
                const ixs: TransactionInstruction[] = [];
                for (let j = 0; j < stepSize; j += 1) {
                    const leafIndex = i + j;
                    const leaf = offChainTree.leaves[leafIndex].node;
                    const verifyIx = createVerifyLeafIx(cmt, {
                        leaf,
                        leafIndex,
                        proof: [],
                        root: splCMT.getCurrentRoot(),
                    });
                    ixs.push(verifyIx);
                }
                i += stepSize;
                await execute(provider, ixs, [payerKeypair], true);
            }
        });
        it('ReplaceLeaf works with no proof accounts', async () => {
            for (let i = 0; i < 2 ** MAX_DEPTH; i += 1) {
                const proof = offChainTree.getProof(i);

                // Replace the current leaf to random bytes, without any additional proof accounts
                const newLeaf = crypto.randomBytes(32);
                const replaceIx = createReplaceIx(cmt, payer, newLeaf, {
                    ...proof,
                    proof: [],
                });
                offChainTree.updateLeaf(i, newLeaf);
                await execute(provider, [replaceIx], [payerKeypair], true, false);

                // Check that replaced leaf actually exists in new tree root
                const splCMT = await ConcurrentMerkleTreeAccount.fromAccountAddress(connection, cmt, {
                    commitment: 'confirmed',
                });
                assert(splCMT.getCurrentRoot().equals(Buffer.from(offChainTree.root)), 'Roots do not match');
            }
        });
    });

    describe('Test integrated appends & replaces', () => {
        beforeEach(async () => {
            [cmtKeypair, offChainTree] = await createTreeOnChain(
                provider,
                payerKeypair,
                0, // Start off with 0 leaves
                DEPTH_SIZE_PAIR,
                MAX_DEPTH // Store full tree on chain
            );
            cmt = cmtKeypair.publicKey;
        });

        it('Testing canopy for appends and replaces on a full on chain tree', async () => {
            // Test that the canopy updates properly throughout multiple modifying instructions
            // in the same transaction
            const leaves: Array<number>[] = [];
            let i = 0;
            const stepSize = 4;
            while (i < 2 ** MAX_DEPTH) {
                const ixs: TransactionInstruction[] = [];
                for (let j = 0; j < stepSize; ++j) {
                    const newLeaf = Array.from(Buffer.alloc(32, i + 1));
                    leaves.push(newLeaf);
                    const appendIx = createAppendIx(cmt, payer, newLeaf);
                    ixs.push(appendIx);
                }
                await execute(provider, ixs, [payerKeypair]);
                i += stepSize;
                console.log('Appended', i, 'leaves');
            }

            // Compare on-chain & off-chain roots
            let ixs: TransactionInstruction[] = [];
            const splCMT = await ConcurrentMerkleTreeAccount.fromAccountAddress(connection, cmt);
            const root = splCMT.getCurrentRoot();

            // Test that the entire state of the tree is stored properly
            // by using the canopy to infer proofs to all of the leaves in the tree.
            // We test that the canopy is updating properly by replacing all the leaves
            // in the tree
            const leafList = Array.from(leaves.entries());
            leafList.sort(() => Math.random() - 0.5);
            let replaces = 0;
            const newLeaves: Record<number, Buffer> = {};
            for (const [i, leaf] of leafList) {
                const newLeaf = crypto.randomBytes(32);
                newLeaves[i] = newLeaf;
                const replaceIx = createReplaceIx(cmt, payer, newLeaf, {
                    leaf: Buffer.from(Uint8Array.from(leaf)),
                    leafIndex: i,
                    proof: [],
                    root, // No proof necessary
                });
                ixs.push(replaceIx);
                if (ixs.length == stepSize) {
                    replaces++;
                    await execute(provider, ixs, [payerKeypair], true);
                    console.log('Replaced', replaces * stepSize, 'leaves');
                    ixs = [];
                }
            }

            const newLeafList: Buffer[] = [];
            for (let i = 0; i < 32; ++i) {
                newLeafList.push(newLeaves[i]);
            }

            const tree = new MerkleTree(newLeafList);

            for (let proofSize = 1; proofSize <= 5; ++proofSize) {
                const newLeaf = crypto.randomBytes(32);
                const i = Math.floor(Math.random() * 32);
                const leaf = newLeaves[i];

                let proof = tree.getProof(i);
                const partialProof = proof.proof.slice(0, proofSize);

                // Create an instruction to replace the leaf
                const replaceIx = createReplaceIx(cmt, payer, newLeaf, {
                    ...proof,
                    proof: partialProof,
                });
                tree.updateLeaf(i, newLeaf);

                // Create an instruction to undo the previous replace, but using the now-outdated partialProof
                proof = tree.getProof(i);
                const replaceBackIx = createReplaceIx(cmt, payer, leaf, {
                    ...proof,
                    proof: partialProof,
                });
                tree.updateLeaf(i, leaf);
                await execute(provider, [replaceIx, replaceBackIx], [payerKeypair], true, false);
            }
        });
    });
});
