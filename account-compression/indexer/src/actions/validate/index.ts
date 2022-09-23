import { PublicKey } from "@solana/web3.js";
import { Sqlite3 } from "../../db/sqlite3";
import * as bs58 from 'bs58';
import { hash, emptyNode } from "../../merkle-tree";

/**
 * Validates the state of the Tree
 */
export async function validateTree(
    nftDb: Sqlite3,
    depth: number,
    treeId: PublicKey,
    maxSeq: number | null
) {
    // Todo(ngundotra): make this readable
    let tree = new Map<number, [number, string]>();
    for (const row of await nftDb.getTreeRows(treeId, maxSeq)) {
        tree.set(row.nodeIndex, [row.seq, row.hash]);
    }
    let nodeIdx = 1;
    while (nodeIdx < 1 << depth) {
        if (!tree.has(nodeIdx)) {
            nodeIdx = 1 << (Math.floor(Math.log2(nodeIdx)) + 1);
            continue;
        }
        let expected = tree.get(nodeIdx)[1];
        let left: Buffer, right: Buffer;
        if (tree.has(2 * nodeIdx)) {
            left = bs58.decode(tree.get(2 * nodeIdx)[1]);
        } else {
            left = emptyNode(depth - Math.floor(Math.log2(2 * nodeIdx)));
        }
        if (tree.has(2 * nodeIdx + 1)) {
            right = bs58.decode(tree.get(2 * nodeIdx + 1)[1]);
        } else {
            right = emptyNode(depth - Math.floor(Math.log2(2 * nodeIdx)));
        }
        let actual = bs58.encode(hash(left, right));
        if (expected !== actual) {
            console.log(
                `Node mismatch ${nodeIdx}, expected: ${expected}, actual: ${actual}, left: ${bs58.encode(
                    left
                )}, right: ${bs58.encode(right)}`
            );
            return false;
        }
        ++nodeIdx;
    }
    return true;
}