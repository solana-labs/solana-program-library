import { AnchorProvider } from '@project-serum/anchor';
import NodeWallet from '@project-serum/anchor/dist/cjs/nodewallet';
import { Connection, Keypair, PublicKey } from '@solana/web3.js';
import { assert } from 'chai';
import * as crypto from 'crypto';

import { ConcurrentMerkleTreeAccount, createAppendIx, ValidDepthSizePair } from '../../src';
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
    });
    describe(`Having created a tree with 8 leaves`, () => {
        beforeEach(async () => {
            [cmtKeypair, offChainTree] = await createTreeOnChain(provider, payerKeypair, 2 ** 3, {
                maxBufferSize: 8,
                maxDepth: 3,
            });
            cmt = cmtKeypair.publicKey;
        });
        it(`Attempt to append a leaf to a full tree`, async () => {
            // Ensure that this fails
            const newLeaf = crypto.randomBytes(32);
            const appendIx = createAppendIx(cmt, payer, newLeaf);

            try {
                await execute(provider, [appendIx], [payerKeypair]);
                throw Error(
                    'This append instruction should have failed because there is no more space to append leaves'
                );
            } catch (_e) {
                assert(true);
            }
        });
    });
});
