import { AnchorProvider } from '@project-serum/anchor';
import NodeWallet from '@project-serum/anchor/dist/cjs/nodewallet';
import { Connection, Keypair, PublicKey } from '@solana/web3.js';
import { assert } from 'chai';
import * as crypto from 'crypto';

import { ConcurrentMerkleTreeAccount, createReplaceIx, createTransferAuthorityIx, ValidDepthSizePair } from '../../src';
import { MerkleTree } from '../../src/merkle-tree';
import { createTreeOnChain, execute } from '../utils';

describe(`TransferAuthority instruction`, () => {
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

    describe('Unit test transferAuthority', () => {
        const authorityKeypair = Keypair.generate();
        const authority = authorityKeypair.publicKey;
        const randomSignerKeypair = Keypair.generate();
        const randomSigner = randomSignerKeypair.publicKey;

        beforeEach(async () => {
            await provider.connection.confirmTransaction(
                await (connection as Connection).requestAirdrop(authority, 1e10)
            );
            [cmtKeypair, offChainTree] = await createTreeOnChain(provider, authorityKeypair, 1, DEPTH_SIZE_PAIR);
            cmt = cmtKeypair.publicKey;
        });
        it('Attempting to replace with random authority fails', async () => {
            const newLeaf = crypto.randomBytes(32);
            const replaceIndex = 0;
            const proof = offChainTree.getProof(replaceIndex);
            const replaceIx = createReplaceIx(cmt, randomSigner, newLeaf, proof);

            try {
                await execute(provider, [replaceIx], [randomSignerKeypair]);
                assert(false, 'Transaction should have failed since incorrect authority cannot execute replaces');
            } catch {
                assert(true);
            }
        });
        it('Can transfer authority', async () => {
            const transferAuthorityIx = createTransferAuthorityIx(cmt, authority, randomSigner);
            await execute(provider, [transferAuthorityIx], [authorityKeypair]);

            const splCMT = await ConcurrentMerkleTreeAccount.fromAccountAddress(connection, cmt);

            assert(
                splCMT.getAuthority().equals(randomSigner),
                `Upon transfering authority, authority should be ${randomSigner.toString()}, but was instead updated to ${splCMT.getAuthority()}`
            );

            // Attempting to replace with new authority now works
            const newLeaf = crypto.randomBytes(32);
            const replaceIndex = 0;
            const proof = offChainTree.getProof(replaceIndex);
            const replaceIx = createReplaceIx(cmt, randomSigner, newLeaf, proof);

            await execute(provider, [replaceIx], [randomSignerKeypair]);
        });
    });
});
