import { AnchorProvider } from '@project-serum/anchor';
import NodeWallet from '@project-serum/anchor/dist/cjs/nodewallet';
import { Connection, Keypair, PublicKey, TransactionInstruction } from '@solana/web3.js';
import { assert } from 'chai';
import * as crypto from 'crypto';

import { ConcurrentMerkleTreeAccount, createReplaceIx, ValidDepthSizePair } from '../../src';
import { MerkleTree } from '../../src/merkle-tree';
import { createTreeOnChain, execute } from '../utils';

describe(`ReplaceLeaf instruction`, () => {
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
    describe(`Having created a tree with ${MAX_BUFFER_SIZE} leaves`, () => {
        const NUM_LEAVES_TO_REPLACE = MAX_BUFFER_SIZE;

        beforeEach(async () => {
            [cmtKeypair, offChainTree] = await createTreeOnChain(
                provider,
                payerKeypair,
                NUM_LEAVES_TO_REPLACE,
                DEPTH_SIZE_PAIR
            );
            cmt = cmtKeypair.publicKey;
        });

        it(`Replace all of them with same proof information`, async () => {
            // Replace MAX_BUFFER_SIZE leaves before syncing off-chain tree with on-chain tree
            // This is meant to simulate the the case where the Proof Server has fallen behind by MAX_BUFFER_SIZE updates
            const ixArray: TransactionInstruction[] = [];
            const txList: Promise<string>[] = [];

            const leavesToUpdate: Buffer[] = [];
            for (let i = 0; i < NUM_LEAVES_TO_REPLACE; i++) {
                const index = i;
                const newLeaf = crypto.randomBytes(32);
                leavesToUpdate.push(newLeaf);

                const proof = offChainTree.getProof(index);
                const replaceIx = createReplaceIx(cmt, payer, newLeaf, proof);
                ixArray.push(replaceIx);
            }

            // Execute all replaces
            ixArray.map(ix => {
                txList.push(execute(provider, [ix], [payerKeypair]));
            });
            await Promise.all(txList);

            leavesToUpdate.map((leaf, index) => {
                offChainTree.updateLeaf(index, leaf);
            });

            // Compare on-chain & off-chain roots
            const splCMT = await ConcurrentMerkleTreeAccount.fromAccountAddress(connection, cmt);
            const onChainRoot = splCMT.getCurrentRoot();

            assert(
                Buffer.from(onChainRoot).equals(offChainTree.root),
                'Updated on chain root does not match root of updated off chain tree'
            );
        });
    });

    describe(`Having created a tree with 8 leaves`, () => {
        beforeEach(async () => {
            [cmtKeypair, offChainTree] = await createTreeOnChain(provider, payerKeypair, 2 ** 3, {
                maxBufferSize: 8,
                maxDepth: 3,
            });
            cmt = cmtKeypair.publicKey;
        });
        it(`Attempt to replace a leaf beyond the tree's capacity`, async () => {
            // Ensure that this fails
            const outOfBoundsIndex = 8;
            const index = outOfBoundsIndex;
            const newLeaf = crypto.randomBytes(32);

            const node = offChainTree.leaves[outOfBoundsIndex - 1].node;
            const proof = offChainTree.getProof(index - 1).proof;

            const replaceIx = createReplaceIx(cmt, payer, newLeaf, {
                leaf: node,
                leafIndex: index,
                proof,
                root: offChainTree.root,
            });

            try {
                await execute(provider, [replaceIx], [payerKeypair]);
                throw Error('This replace instruction should have failed because the leaf index is out of bounds');
            } catch (_e) {
                assert(true);
            }
        });
    });
});
