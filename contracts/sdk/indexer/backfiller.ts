import { PublicKey, SIGNATURE_LENGTH_IN_BYTES } from "@solana/web3.js";
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

function onlyUnique(value, index, self) {
  return self.indexOf(value) === index;
}

export async function getAllTreeSlots(
  connection: Connection,
  treeId: string,
  afterSig?: string
): Promise<number[]> {
  const treeAddress = new PublicKey(treeId);
  // todo: paginate
  let lastAddress: string | null = null;
  let done = false;
  const history: number[] = [];

  const baseOpts = afterSig ? { until: afterSig } : {};
  while (!done) {
    let opts = lastAddress ? { before: lastAddress } : {};
    const finalOpts = { ...baseOpts, ...opts };
    console.log(finalOpts);
    const sigs = await connection.getSignaturesForAddress(treeAddress, finalOpts);
    console.log(sigs[sigs.length - 1]);
    lastAddress = sigs[sigs.length - 1].signature;
    sigs.map((sigInfo) => {
      history.push(sigInfo.slot);
    })

    if (sigs.length < 1000) {
      done = true;
    }
  }

  return history.reverse().filter(onlyUnique);
}

/// Returns tree history in chronological order (oldest first)
export async function backfillTreeHistory(
  connection: Connection,
  nftDb: NFTDatabaseConnection,
  parserState: ParserState,
  treeId: string,
  startSeq: number,
  fromSlot: number | null,
): Promise<number> {
  const treeAddress = new PublicKey(treeId);
  const merkleRoll = decodeMerkleRoll(await (await connection.getAccountInfo(treeAddress)).data);
  const maxSeq = merkleRoll.roll.sequenceNumber.toNumber();
  // Sequence number on-chain is ready to setup the 
  if (startSeq === maxSeq - 1) {
    return startSeq;
  }
  const earliestTxId = await nftDb.getTxIdForSlot(treeId, fromSlot);
  console.log("Tx id:", earliestTxId);
  const treeHistory = await getAllTreeSlots(connection, treeId, earliestTxId);
  console.log("Retrieved tree history!", treeHistory);

  let numProcessed = 0;
  let batchSize = 20;
  while (numProcessed < treeHistory.length) {
    const batchJobs = [];
    for (let i = 0; i < batchSize; i++) {
      const historyIndex = numProcessed + i;
      if (historyIndex >= treeHistory.length) {
        break;
      }
      batchJobs.push(
        plugGapsFromSlot(
          connection,
          nftDb,
          parserState,
          treeAddress,
          treeHistory[historyIndex],
          0,
          maxSeq,
        )
      )
    }
    await Promise.all(batchJobs);
    numProcessed += batchJobs.length;
    console.log("num processed: ", numProcessed);
  }
  return maxSeq;
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
