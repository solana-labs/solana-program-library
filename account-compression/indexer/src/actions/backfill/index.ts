import { ConcurrentMerkleTreeAccount } from "@solana/spl-account-compression"
import { Connection, PublicKey } from "@solana/web3.js"
import { Sqlite3 } from "../../db/sqlite3"
import { ingestTransaction } from "../../ingest"
import { GapInfo } from "../../ingest/types/Gap"
import { BlockTransaction, getTransactionsFromBlock } from "../fetch"
import { getAllSlotsMissing } from "../fetch/getAllSlotsMissing"
import { validateTree } from "../validate"

export type BackfillArguments = {
    treeId: PublicKey,
    tableName: string,
    rpcUrl: string
}

async function getLatestSlotForAddress(connection: Connection, address: PublicKey): Promise<{
    slot: number,
    transactionId: string,
}> {
    const sigs = await connection.getSignaturesForAddress(address, undefined, "confirmed");
    return { slot: sigs[0].slot, transactionId: sigs[0].signature }
}

async function getFirstTransactionSigInBlock(connection: Connection, slot: number): Promise<string> {
    return (await connection.getBlockSignatures(slot, "confirmed")).signatures[0]
}

export async function backfill(args: BackfillArguments) {
    // Create RPC connection
    const connection = new Connection(args.rpcUrl);
    const cmt = await ConcurrentMerkleTreeAccount.fromAccountAddress(connection, args.treeId);

    // Create db connection
    const db = new Sqlite3({
        tableDir: "./db",
        reset: true
    });

    // Initialize db connection
    try {
        await db.bootstrap();
    } catch (e) {
        await db.bootstrap();
    }

    // Find starting spot, and a queue of gaps to index
    let gaps: GapInfo[];

    let latest = await getLatestSlotForAddress(connection, args.treeId);
    let currentCMTSeq = cmt.getCurrentSeq() - 1;
    console.log(currentCMTSeq);
    if (!await db.hasCMT(args.treeId)) {
        gaps = [];
        let firstTransactionId: string = await getFirstTransactionSigInBlock(connection, cmt.getCreationSlot());

        // Our single gap is the entire history of the tree
        gaps.push({
            previousSeq: 0,
            previousSlot: cmt.getCreationSlot(),
            previousTransactionId: firstTransactionId,
            currentSeq: currentCMTSeq + 1,
            currentSlot: latest.slot,
            currentTransactionId: latest.transactionId,
        })
    } else {
        // Calculate gaps (if any) in our current database
        gaps = await db.getMissingData(args.treeId);

        // If our database is behind the chain, add a gap
        const latestTreeInfo = await db.getLatestTreeInfo(args.treeId);
        if (latestTreeInfo.seq < currentCMTSeq) {
            gaps.push({
                previousSeq: latestTreeInfo.seq,
                previousSlot: latestTreeInfo.slot,
                previousTransactionId: latestTreeInfo.transactionId,
                currentSeq: currentCMTSeq,
                currentSlot: latest.slot,
                currentTransactionId: latest.transactionId,
            })
        }
    }
    console.log(gaps);

    // Calculate # slots to index
    const slots = await getAllSlotsMissing(connection, args.treeId, gaps);
    console.log(slots);
    if (gaps[gaps.length - 1].currentSeq <= currentCMTSeq + 1) {
        slots.push(latest.slot);
    }

    for (const slot of slots) {
        // Fetch slot & parse transactions
        const slotTransactions = await getTransactionsFromBlock(connection, slot, args.treeId);
        console.log(`Slot ${slot} has ${slotTransactions.length} transaction(s)`);

        // Parse transaction data & insert to database
        for (const transaction of slotTransactions) {
            await ingestTransaction(db, slot, transaction);
        }
    }

    // validate state
    const valid = await validateTree(db, cmt.getMaxDepth(), args.treeId, currentCMTSeq + 1);
    if (valid) {
        console.log(`✅ Tree state for ${args.treeId.toBase58()} is valid!`)
    } else {
        console.error(`❌ Tree state for ${args.treeId.toBase58()} is not valid. Please run again to see if this can be repaired`);
    }

}