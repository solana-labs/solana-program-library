import {
    Connection,
    PublicKey,
    Transaction,
    TransactionInstruction,
    Keypair,
    Signer,
} from '@solana/web3.js';
import {
    AnchorProvider
} from '@project-serum/anchor';
import { assert } from 'chai';
import * as crypto from "crypto";

import {
    ConcurrentMerkleTreeAccount,
    createInitEmptyMerkleTreeIx,
    createAllocTreeIx,
    createAppendIx,
} from '../src'
import {
    buildTree,
    Tree,
} from "./merkleTree";

export async function assertCMTProperties(
    connection: Connection,
    expectedMaxDepth: number,
    expectedMaxBufferSize: number,
    expectedAuthority: PublicKey,
    expectedRoot: Buffer,
    onChainCMTKey: PublicKey
) {
    const onChainCMT = await ConcurrentMerkleTreeAccount.fromAccountAddress(connection, onChainCMTKey);

    assert(
        onChainCMT.getMaxDepth() === expectedMaxDepth,
        `Max depth does not match ${onChainCMT.getMaxDepth()}, expected ${expectedMaxDepth}`,
    );
    assert(
        onChainCMT.getMaxBufferSize() === expectedMaxBufferSize,
        `Max buffer size does not match ${onChainCMT.getMaxBufferSize()}, expected ${expectedMaxBufferSize}`,
    );
    assert(
        onChainCMT.getAuthority().equals(expectedAuthority),
        "Failed to write auth pubkey",
    );
    assert(
        onChainCMT.getCurrentRoot().equals(expectedRoot),
        "On chain root does not match root passed in instruction",
    );
}


/// Wait for a transaction of a certain id to confirm and optionally log its messages
export async function confirmAndLogTx(provider: AnchorProvider, txId: string, verbose: boolean = false) {
    const tx = await provider.connection.confirmTransaction(txId, "confirmed");
    if (tx.value.err || verbose) {
        console.log(
            (await provider.connection.getConfirmedTransaction(txId, "confirmed"))!.meta!
                .logMessages
        );
    }
    if (tx.value.err) {
        console.log("Transaction failed");
        throw new Error(JSON.stringify(tx.value.err));
    }
};

/// Execute a series of instructions in a txn
export async function execute(
    provider: AnchorProvider,
    instructions: TransactionInstruction[],
    signers: Signer[],
    skipPreflight: boolean = false,
    verbose: boolean = false,
): Promise<string> {
    let tx = new Transaction();
    instructions.map((ix) => { tx = tx.add(ix) });

    let txid: string | null = null;
    try {
        txid = await provider.sendAndConfirm!(tx, signers, {
            skipPreflight,
        })
    } catch (e: any) {
        console.log("Tx error!", e.logs)
        throw e;
    }

    if (verbose && txid) {
        console.log(
            (await provider.connection.getConfirmedTransaction(txid, "confirmed"))!.meta!
                .logMessages
        );
    }

    return txid;
}

export async function createTreeOnChain(
    provider: AnchorProvider,
    payer: Keypair,
    numLeaves: number,
    maxDepth: number,
    maxSize: number,
    canopyDepth: number = 0,
): Promise<[Keypair, Tree]> {
    const cmtKeypair = Keypair.generate();

    const leaves = Array(2 ** maxDepth).fill(Buffer.alloc(32));
    for (let i = 0; i < numLeaves; i++) {
        leaves[i] = crypto.randomBytes(32);
    }
    const tree = buildTree(leaves);

    const allocAccountIx = await createAllocTreeIx(
        provider.connection,
        maxSize,
        maxDepth,
        canopyDepth,
        payer.publicKey,
        cmtKeypair.publicKey
    );

    let ixs = [
        allocAccountIx,
        createInitEmptyMerkleTreeIx(
            payer,
            cmtKeypair.publicKey,
            maxDepth,
            maxSize,
        )
    ];

    let txId = await execute(provider, ixs, [
        payer,
        cmtKeypair,
    ]);
    if (canopyDepth) {
        await confirmAndLogTx(provider, txId as string);
    }

    if (numLeaves) {
        const nonZeroLeaves = leaves.slice(0, numLeaves);
        let appendIxs: TransactionInstruction[] = nonZeroLeaves.map((leaf) => {
            return createAppendIx(leaf, payer, cmtKeypair.publicKey)
        });
        while (appendIxs.length) {
            const batch = appendIxs.slice(0, 5);
            await execute(provider, batch, [payer]);
            appendIxs = appendIxs.slice(5,);
        }
    }

    await assertCMTProperties(
        provider.connection,
        maxDepth,
        maxSize,
        payer.publicKey,
        tree.root,
        cmtKeypair.publicKey
    );

    return [cmtKeypair, tree];
}