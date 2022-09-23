import { PublicKey } from "@solana/web3.js";
import { Sqlite3 } from "../db/sqlite3";
import { Proof } from "./types";
import { emptyNode } from "./emptyNode";
import { generateRoot } from "./generateRoot";
import * as bs58 from 'bs58';

export async function getMerkleProof(
    db: Sqlite3,
    treeId: PublicKey,
    nodeIndex: number,
    maxSequenceNumber: number,
): Promise<Proof> {
    // Create list of nodes we need to construct a proof
    const nodes: number[] = [];
    let n = nodeIndex;
    while (n > 1) {
        if (n % 2 == 0) {
            nodes.push(n + 1);
        } else {
            nodes.push(n - 1);
        }
        n >>= 1;
    }
    // We fetch the root to figure out the maxDepth of the tree
    nodes.push(1);
    // We fetch leaf to get most recent leaf hash
    nodes.push(nodeIndex);

    // Retrieve proof nodes
    let res = await db.getTreeRowsForNodes(
        treeId,
        nodes,
        maxSequenceNumber,
    )
    console.log(res);

    // Remove the root node from proof array
    let root = res.pop();
    // Check that root node was actually returned
    if (root.nodeIndex != 1) {
        throw Error(`Root node was not returned by query. Highest node in the tree returned has index: ${root.nodeIndex}`);
    }

    // Remove the leaf node from proof array
    const indexToPop = res.findIndex((row) => row.nodeIndex === nodeIndex);
    const leafRow = res[indexToPop];
    res = res.slice(0, indexToPop).concat(res.slice(indexToPop + 1,));

    // Generate a default proof of empty nodes of length `maxDepth`
    const proof: string[] = [];
    for (let i = 0; i < root.level; i++) {
        proof.push(bs58.encode(emptyNode(i)));
    }
    // Replace default proof with nodes in database
    for (const node of res) {
        proof[node.level] = node.hash;
    }

    // Convert nodeIndex from tree-space to leaf-space
    let leafIdx = nodeIndex - (1 << root.level);

    let inferredProof = {
        leaf: leafRow.hash,
        root: root.hash,
        proofNodes: proof,
        index: leafIdx,
    };

    // Always regenerate root to validate proof :)
    inferredProof.root = bs58.encode(generateRoot(inferredProof));
    return inferredProof;
}