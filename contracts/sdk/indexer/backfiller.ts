import { PublicKey } from "@solana/web3.js";
import { Connection } from "@solana/web3.js";
import { decodeMerkleRoll } from "../gummyroll/index";
import { ParserState, handleLogsAtomic } from "./indexer/utils";
import { hash, NFTDatabaseConnection } from "./db";
import { bs58 } from "@project-serum/anchor/dist/cjs/utils/bytes";

export async function validateTree(
  nftDb: NFTDatabaseConnection,
  depth: number,
  treeId: string,
  maxSeq: number | null
) {
  let tree = new Map<number, [number, string]>();
  for (const row of await nftDb.getTree(treeId, maxSeq)) {
    tree.set(row.node_idx, [row.seq, row.hash]);
  }
  let nodeIdx = 1;
  while (nodeIdx < 1 << depth) {
    if (!tree.has(nodeIdx)) {
      // Just trust, bro
      nodeIdx = 1 << (Math.floor(Math.log2(nodeIdx)) + 1);
      continue;
    }
    let expected = tree.get(nodeIdx)[1];
    let left, right;
    if (tree.has(2 * nodeIdx)) {
      left = bs58.decode(tree.get(2 * nodeIdx)[1]);
    } else {
      left = nftDb.emptyNode(depth - Math.floor(Math.log2(2 * nodeIdx)));
    }
    if (tree.has(2 * nodeIdx + 1)) {
      right = bs58.decode(tree.get(2 * nodeIdx + 1)[1]);
    } else {
      right = nftDb.emptyNode(depth - Math.floor(Math.log2(2 * nodeIdx)));
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

async function plugGapsFromSlot(
  connection: Connection,
  nftDb: NFTDatabaseConnection,
  parserState: ParserState,
  treeKey: PublicKey,
  slot: number,
  startSeq: number,
  endSeq: number
) {
  const blockData = await connection.getBlock(slot, {
    commitment: "confirmed",
  });
  for (const tx of blockData.transactions) {
    if (
      tx.transaction.message
        .programIds()
        .every((pk) => !pk.equals(parserState.Bubblegum.programId))
    ) {
      continue;
    }
    if (tx.transaction.message.accountKeys.every((pk) => !pk.equals(treeKey))) {
      continue;
    }
    if (tx.meta.err) {
      continue;
    }
    handleLogsAtomic(
      nftDb,
      {
        err: null,
        logs: tx.meta.logMessages,
        signature: tx.transaction.signatures[0],
      },
      { slot: slot },
      parserState,
      startSeq,
      endSeq
    );
  }
}

async function plugGaps(
  connection: Connection,
  nftDb: NFTDatabaseConnection,
  parserState: ParserState,
  treeId: string,
  startSlot: number,
  endSlot: number,
  startSeq: number,
  endSeq: number
) {
  const treeKey = new PublicKey(treeId);
  for (let slot = startSlot; slot <= endSlot; ++slot) {
    await plugGapsFromSlot(
      connection,
      nftDb,
      parserState,
      treeKey,
      slot,
      startSeq,
      endSeq
    );
  }
}

export async function fetchAndPlugGaps(
  connection: Connection,
  nftDb: NFTDatabaseConnection,
  minSeq: number,
  treeId: string,
  parserState: ParserState
) {
  let [missingData, maxDbSeq, maxDbSlot] = await nftDb.getMissingData(
    minSeq,
    treeId
  );
  console.log(`Found ${missingData.length} gaps`);
  let currSlot = await connection.getSlot("confirmed");

  let merkleAccount = await connection.getAccountInfo(
    new PublicKey(treeId),
    "confirmed"
  );
  if (!merkleAccount) {
    return;
  }
  let merkleRoll = decodeMerkleRoll(merkleAccount.data);
  let merkleSeq = merkleRoll.roll.sequenceNumber.toNumber() - 1;

  if (merkleSeq - maxDbSeq > 1 && maxDbSlot < currSlot) {
    console.log("Running forward filler");
    missingData.push({
      prevSeq: maxDbSeq,
      currSeq: merkleSeq,
      prevSlot: maxDbSlot,
      currSlot: currSlot,
    });
  }

  for (const { prevSeq, currSeq, prevSlot, currSlot } of missingData) {
    console.log(prevSeq, currSeq, prevSlot, currSlot);
    await plugGaps(
      connection,
      nftDb,
      parserState,
      treeId,
      prevSlot,
      currSlot,
      prevSeq,
      currSeq
    );
  }
  console.log("Done");
  return maxDbSeq;
}
