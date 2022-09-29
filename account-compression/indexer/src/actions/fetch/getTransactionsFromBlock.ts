import { Sqlite3 } from "../../db/sqlite3";
import {
    Connection,
    PublicKey,
    VersionedMessage,
    ConfirmedTransactionMeta,
    TransactionVersion,
} from '@solana/web3.js'
import { SPL_ACCOUNT_COMPRESSION_PROGRAM_ID, SPL_NOOP_PROGRAM_ID } from "@solana/spl-account-compression";

export type BlockTransaction = {
    /** The transaction */
    transaction: {
        /** The transaction message */
        message: VersionedMessage;
        /** The transaction signatures */
        signatures: string[];
    };
    /** Metadata produced from the transaction */
    meta: ConfirmedTransactionMeta | null;
    /** The transaction version */
    version?: TransactionVersion;
}

export function getTransactionSignature(transaction: BlockTransaction): string {
    return transaction.transaction.signatures[0];
}

/// Filters the transactions in a block
export async function getTransactionsFromBlock(
    connection: Connection,
    slot: number,
    treeId: PublicKey,
    startSeq?: number,
    endSeq?: number,
): Promise<BlockTransaction[]> {
    const blockData = await connection.getBlock(slot, {
        commitment: "confirmed",
        maxSupportedTransactionVersion: 2,
    });
    if (!blockData) {
        return [];
    }

    let transactions: BlockTransaction[] = [];
    for (const tx of blockData.transactions) {
        if (tx.meta!.err) {
            continue;
        }

        // Check keys
        const accountKeys = tx.transaction.message.getAccountKeys();
        let foundTree = false;
        let foundCompression = false;
        let foundNoop = false;
        let relevant = false;
        for (let i = 0; !relevant && i < accountKeys.length; i++) {
            const key = accountKeys.get(i);
            if (key && key.equals(treeId)) {
                foundTree = true;
            } else if (key && key.equals(SPL_ACCOUNT_COMPRESSION_PROGRAM_ID)) {
                foundCompression = true;
            } else if (key && key.equals(SPL_NOOP_PROGRAM_ID)) {
                foundNoop = true;
            }
            relevant = foundTree && foundCompression && foundNoop;
        }
        if (!relevant) {
            continue;
        }
        transactions.push(tx);
    }

    return transactions;
}