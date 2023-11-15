import { AnchorProvider } from '@project-serum/anchor';
import NodeWallet from '@project-serum/anchor/dist/cjs/nodewallet';
import { Connection, Keypair, PublicKey, TransactionInstruction } from '@solana/web3.js';
import { assert } from 'chai';

import { createCloseEmptyTreeInstruction, createReplaceIx, ValidDepthSizePair } from '../../src';
import { MerkleTree } from '../../src/merkle-tree';
import { createTreeOnChain, execute } from '../utils';

describe(`CloseEmptyTree instruction`, () => {
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

    describe('Testing execution', () => {
        const NUM_LEAVES_TO_EMPTY = 10;

        beforeEach(async () => {
            [cmtKeypair, offChainTree] = await createTreeOnChain(
                provider,
                payerKeypair,
                NUM_LEAVES_TO_EMPTY,
                DEPTH_SIZE_PAIR
            );
            cmt = cmtKeypair.publicKey;
        });

        it('Empty all of the leaves and close the tree', async () => {
            // Empty all the leaves in the tree
            const ixArray: TransactionInstruction[] = [];
            const txList: Promise<string>[] = [];
            const leavesToUpdate: Buffer[] = [];
            for (let i = 0; i < NUM_LEAVES_TO_EMPTY; i++) {
                const index = i;
                const newLeaf = Buffer.alloc(32);
                leavesToUpdate.push(newLeaf);

                const proof = offChainTree.getProof(index);
                const replaceIx = createReplaceIx(cmt, payer, Buffer.alloc(32), proof);
                ixArray.push(replaceIx);
            }
            ixArray.map(ix => {
                txList.push(execute(provider, [ix], [payerKeypair]));
            });
            await Promise.all(txList);

            // Check that the user is able to close the tree and receive all passports
            let payerInfo = await connection.getAccountInfo(payer, 'confirmed');
            let treeInfo = await connection.getAccountInfo(cmt, 'confirmed');
            if (payerInfo === null) {
                assert(false, 'Expected payer to exist');
                return;
            }
            if (treeInfo === null) {
                assert(false, 'Expected tree to exist');
                return;
            }
            const payerLamports = payerInfo.lamports;
            const treeLamports = treeInfo.lamports;

            const ix = createCloseEmptyTreeInstruction({
                authority: payer,
                merkleTree: cmt,
                recipient: payer,
            });
            await execute(provider, [ix], [payerKeypair]);

            payerInfo = await provider.connection.getAccountInfo(payer, 'confirmed');
            if (payerInfo === null) {
                assert(false, 'Expected payer to exist');
                return;
            }

            const finalLamports = payerInfo.lamports;
            assert(
                finalLamports === payerLamports + treeLamports - 5000,
                'Expected payer to have received the lamports from the closed tree account'
            );

            treeInfo = await provider.connection.getAccountInfo(cmt, 'confirmed');
            assert(treeInfo === null, 'Expected the merkle tree account info to be null');
        });
        it('It cannot be closed until empty', async () => {
            const ix = createCloseEmptyTreeInstruction({
                authority: payer,
                merkleTree: cmt,
                recipient: payer,
            });
            try {
                await execute(provider, [ix], [payerKeypair]);
                assert(false, 'Closing a tree account before it is empty should ALWAYS error');
            } catch (e) {
                assert(true);
            }
        });
    });
});
