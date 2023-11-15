import { AnchorProvider } from '@project-serum/anchor';
import NodeWallet from '@project-serum/anchor/dist/cjs/nodewallet';
import { Connection, Keypair, PublicKey } from '@solana/web3.js';
import { assert } from 'chai';
import * as crypto from 'crypto';

import {
    ConcurrentMerkleTreeAccount,
    createAppendIx,
    createReplaceIx,
    createVerifyLeafIx,
    ValidDepthSizePair,
} from '../src';
import { MerkleTree } from '../src/merkle-tree';
import { createTreeOnChain, execute } from './utils';

describe('Account Compression', () => {
    // Configure the client to use the local cluster.
    let offChainTree: MerkleTree;
    let cmtKeypair: Keypair;
    let cmt: PublicKey;
    let payerKeypair: Keypair;
    let payer: PublicKey;
    let connection: Connection;
    let provider: AnchorProvider;

    const MAX_SIZE = 64;
    const MAX_DEPTH = 14;
    const DEPTH_SIZE_PAIR: ValidDepthSizePair = {
        maxBufferSize: MAX_SIZE,
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

    describe('Having created a tree with a single leaf', () => {
        beforeEach(async () => {
            [cmtKeypair, offChainTree] = await createTreeOnChain(provider, payerKeypair, 1, DEPTH_SIZE_PAIR);
            cmt = cmtKeypair.publicKey;
        });
        it('Append single leaf', async () => {
            const newLeaf = crypto.randomBytes(32);
            const appendIx = createAppendIx(cmt, payer, newLeaf);

            await execute(provider, [appendIx], [payerKeypair]);
            offChainTree.updateLeaf(1, newLeaf);

            const splCMT = await ConcurrentMerkleTreeAccount.fromAccountAddress(connection, cmt);
            const onChainRoot = splCMT.getCurrentRoot();

            assert(
                Buffer.from(onChainRoot).equals(offChainTree.root),
                'Updated on chain root matches root of updated off chain tree'
            );
        });
        it('Verify proof works for that leaf', async () => {
            const newLeaf = crypto.randomBytes(32);
            const index = 0;
            const proof = offChainTree.getProof(index);

            const verifyLeafIx = createVerifyLeafIx(cmt, proof);
            const replaceLeafIx = createReplaceIx(cmt, payer, newLeaf, proof);
            await execute(provider, [verifyLeafIx, replaceLeafIx], [payerKeypair]);

            offChainTree.updateLeaf(index, newLeaf);

            const splCMT = await ConcurrentMerkleTreeAccount.fromAccountAddress(connection, cmt);
            const onChainRoot = splCMT.getCurrentRoot();

            assert(
                Buffer.from(onChainRoot).equals(offChainTree.root),
                'Updated on chain root matches root of updated off chain tree'
            );
        });
        it('Verify leaf fails when proof fails', async () => {
            const newLeaf = crypto.randomBytes(32);
            const index = 0;
            // Replace valid proof with random bytes so it is wrong
            const proof = offChainTree.getProof(index);
            proof.proof = proof.proof.map(_ => {
                return crypto.randomBytes(32);
            });

            // Verify proof is invalid
            const verifyLeafIx = createVerifyLeafIx(cmt, proof);
            try {
                await execute(provider, [verifyLeafIx], [payerKeypair]);
                assert(false, 'Proof should have failed to verify');
            } catch {}

            // Replace instruction with same proof fails
            const replaceLeafIx = createReplaceIx(cmt, payer, newLeaf, proof);
            try {
                await execute(provider, [replaceLeafIx], [payerKeypair]);
                assert(false, 'Replace should have failed to verify');
            } catch {}

            const splCMT = await ConcurrentMerkleTreeAccount.fromAccountAddress(connection, cmtKeypair.publicKey);
            const onChainRoot = splCMT.getCurrentRoot();

            assert(
                Buffer.from(onChainRoot).equals(offChainTree.root),
                'Updated on chain root matches root of updated off chain tree'
            );
        });
        it('Replace that leaf', async () => {
            const newLeaf = crypto.randomBytes(32);
            const index = 0;

            const replaceLeafIx = createReplaceIx(cmt, payer, newLeaf, offChainTree.getProof(index));
            assert(replaceLeafIx.keys.length == 3 + MAX_DEPTH, `Failed to create proof for ${MAX_DEPTH}`);

            await execute(provider, [replaceLeafIx], [payerKeypair]);

            offChainTree.updateLeaf(index, newLeaf);

            const splCMT = await ConcurrentMerkleTreeAccount.fromAccountAddress(connection, cmt);
            const onChainRoot = splCMT.getCurrentRoot();

            assert(
                Buffer.from(onChainRoot).equals(offChainTree.root),
                'Updated on chain root matches root of updated off chain tree'
            );
        });

        it('Replace that leaf with while autocompleting a proof', async () => {
            /*
             In this test, we only pass 1 node of the proof to the instruction, and the rest is autocompleted.
             This test should pass because we just previously appended the leaf, so the proof can be autocompleted
             from what is in the active buffer.
             There probably isn't a use case for this, but it is still expected behavior, hence this test.
            */

            const newLeaf = crypto.randomBytes(32);
            const index = 0;

            // Truncate proof to only top most node
            const proof = offChainTree.getProof(index, true, 1);
            assert(proof.proof.length === 1, 'Failed to minimize proof to expected size of 1');

            const replaceLeafIx = createReplaceIx(cmt, payer, newLeaf, proof);
            await execute(provider, [replaceLeafIx], [payerKeypair]);

            offChainTree.updateLeaf(index, newLeaf);

            const splCMT = await ConcurrentMerkleTreeAccount.fromAccountAddress(connection, cmt);
            const onChainRoot = splCMT.getCurrentRoot();

            assert(
                Buffer.from(onChainRoot).equals(offChainTree.root),
                'Updated on chain root matches root of updated off chain tree'
            );
        });
    });

    describe(`Having created a tree with depth 3`, () => {
        const DEPTH = 3;
        beforeEach(async () => {
            [cmtKeypair, offChainTree] = await createTreeOnChain(provider, payerKeypair, 0, {
                maxBufferSize: 8,
                maxDepth: DEPTH,
            });
            cmt = cmtKeypair.publicKey;

            for (let i = 0; i < 2 ** DEPTH; i++) {
                const newLeaf = Array.from(Buffer.alloc(32, i + 1));
                const appendIx = createAppendIx(cmt, payer, newLeaf);
                await execute(provider, [appendIx], [payerKeypair]);
            }

            // Compare on-chain & off-chain roots
            const splCMT = await ConcurrentMerkleTreeAccount.fromAccountAddress(connection, cmt);

            assert(splCMT.getBufferSize() === 2 ** DEPTH, 'Not all changes were processed');
            assert(splCMT.getCurrentBufferIndex() === 0, 'Not all changes were processed');
        });

        it('Random attacker fails to fake the existence of a leaf by autocompleting proof', async () => {
            const maliciousLeafHash = crypto.randomBytes(32);
            const maliciousLeafHash1 = crypto.randomBytes(32);
            const nodeProof: Buffer[] = [];
            for (let i = 0; i < DEPTH; i++) {
                nodeProof.push(Buffer.alloc(32));
            }

            // Root - make this nonsense so it won't match what's in ChangeLog, thus forcing proof autocompletion
            const replaceIx = createReplaceIx(cmt, payer, maliciousLeafHash1, {
                leaf: maliciousLeafHash,
                leafIndex: 0,
                proof: nodeProof,
                root: Buffer.alloc(32),
            });

            try {
                await execute(provider, [replaceIx], [payerKeypair]);
                assert(false, 'Attacker was able to succesfully write fake existence of a leaf');
            } catch (e) {}

            const splCMT = await ConcurrentMerkleTreeAccount.fromAccountAddress(connection, cmt);

            assert(
                splCMT.getCurrentBufferIndex() === 0,
                "CMT updated its active index after attacker's transaction, when it shouldn't have done anything"
            );
        });
    });
});
