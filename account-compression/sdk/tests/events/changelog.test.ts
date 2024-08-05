import { strict as assert } from 'node:assert';

import { AnchorProvider } from '@coral-xyz/anchor';
import NodeWallet from '@coral-xyz/anchor/dist/cjs/nodewallet';
import { bs58 } from '@coral-xyz/anchor/dist/cjs/utils/bytes';
import { Connection, Keypair, PublicKey } from '@solana/web3.js';
import { BN } from 'bn.js';
import * as crypto from 'crypto';

import { createAppendIx, deserializeChangeLogEventV1, SPL_NOOP_PROGRAM_ID } from '../../src';
import { MerkleTree } from '../../src/merkle-tree';
import { createTreeOnChain, execute } from '../utils';

describe('Serde tests', () => {
    let offChainTree: MerkleTree;
    let cmtKeypair: Keypair;
    let payerKeypair: Keypair;
    let payer: PublicKey;
    let connection: Connection;
    let provider: AnchorProvider;

    const MAX_SIZE = 64;
    const MAX_DEPTH = 14;

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
            'confirmed',
        );
    });
    describe('ChangeLogEvent tests', () => {
        let cmt: PublicKey;
        beforeEach(async () => {
            [cmtKeypair, offChainTree] = await createTreeOnChain(provider, payerKeypair, 0, {
                maxBufferSize: MAX_SIZE,
                maxDepth: MAX_DEPTH,
            });
            cmt = cmtKeypair.publicKey;
        });
        it('Can deserialize a ChangeLogEvent', async () => {
            const newLeaf = crypto.randomBytes(32);
            const txId = await execute(provider, [createAppendIx(cmt, payer, newLeaf)], [payerKeypair]);
            offChainTree.updateLeaf(0, newLeaf);

            const transaction = await connection.getTransaction(txId, {
                commitment: 'confirmed',
                maxSupportedTransactionVersion: 2,
            });

            // Get noop program instruction
            const accountKeys = transaction!.transaction.message.getAccountKeys();
            const noopInstruction = transaction!.meta!.innerInstructions![0].instructions[0];
            const programId = accountKeys.get(noopInstruction.programIdIndex)!;
            if (!programId.equals(SPL_NOOP_PROGRAM_ID)) {
                throw Error(`Only inner ix should be a noop, but instead is a ${programId.toBase58()}`);
            }
            const cpiData = Buffer.from(bs58.decode(noopInstruction.data));
            const changeLogEvent = deserializeChangeLogEventV1(cpiData);

            assert(
                changeLogEvent.treeId.equals(cmt),
                `Tree id in changeLog differs from expected tree ${cmt.toBase58()}`,
            );
            assert(changeLogEvent.index === 0, `ChangeLog should have index 0, but has index ${changeLogEvent.index}`);
            assert(
                new BN.BN(changeLogEvent.seq).toNumber() === 1,
                `ChangeLog should have sequence number of 1, but has seq number of ${changeLogEvent.seq.toString()}`,
            );

            // Check that the emitted ChangeLog path matches up with the updated Tree
            let nodeIndex = 0 + (1 << MAX_DEPTH);
            let realTreeNode = offChainTree.leaves[0];
            while (nodeIndex > 0) {
                const clNode = changeLogEvent.path.shift()!;
                assert(
                    nodeIndex === clNode.index,
                    `Expected changeLog index to be ${nodeIndex} but is ${clNode.index}`,
                );
                assert(
                    realTreeNode!.node.equals(Buffer.from(clNode.node)),
                    `ChangeLog node differs at node index: ${clNode.index}`,
                );
                realTreeNode = realTreeNode.parent!;
                nodeIndex = nodeIndex >> 1;
            }
        });
    });
});
