import { PublicKey, SIGNATURE_LENGTH_IN_BYTES } from "@solana/web3.js";
import { Connection } from "@solana/web3.js";
import { decodeMerkleRoll } from "../gummyroll/index";
import { ParserState, handleInstructionsAtomic } from "./indexer/utils";
import { handleLogsAtomic } from "./indexer/log/bubblegum";
import { GapInfo, hash, NFTDatabaseConnection } from "./db";
import { bs58 } from "@project-serum/anchor/dist/cjs/utils/bytes";
import { ParseResult } from "./indexer/utils";

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

/// Inserts data if its in [startSeq, endSeq)
export async function plugGapsFromSlot(
  connection: Connection,
  nftDb: NFTDatabaseConnection,
  parserState: ParserState,
  slot: number,
  startSeq: number,
  endSeq: number,
  treeKey?: PublicKey,
) {
  const blockData = await connection.getBlock(slot, {
    commitment: "confirmed",
  });
  for (const tx of blockData.transactions) {
    if (treeKey && tx.transaction.message.accountKeys.every((pk) => !pk.equals(treeKey) && !pk.equals(parserState.Bubblegum.programId))) {
      continue;
    }
    if (tx.meta.err) {
      continue;
    }

    const parseResult = handleLogsAtomic(
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
    if (parseResult === ParseResult.LogTruncated) {
      const instructionInfo = {
        accountKeys: tx.transaction.message.accountKeys,
        instructions: tx.transaction.message.instructions,
        innerInstructions: tx.meta.innerInstructions,
      }
      handleInstructionsAtomic(
        nftDb,
        instructionInfo,
        tx.transaction.signatures[0],
        { slot: slot },
        parserState,
        startSeq,
        endSeq
      );
    }
  }
}

type BatchedGapRequest = {
  slot: number,
  startSeq: number,
  endSeq: number
};

async function plugGapsFromSlotBatched(
  connection: Connection,
  nftDb: NFTDatabaseConnection,
  parserState: ParserState,
  treeId: string,
  requests: BatchedGapRequest[],
  batchSize: number = 20,
) {
  const treeKey = new PublicKey(treeId);
  let idx = 0;
  while (idx < requests.length) {
    const batchJobs = [];
    for (let i = 0; i < batchSize; i += 1) {
      const requestIdx = idx + i;
      if (requestIdx >= requests.length) { break }
      const request = requests[requestIdx];
      batchJobs.push(
        plugGapsFromSlot(
          connection,
          nftDb,
          parserState,
          request.slot,
          request.startSeq,
          request.endSeq,
          treeKey,
        )
          .catch((e) => {
            console.error(`Failed to plug gap from slot: ${request.slot}`, e);
          })
      );
    }
    await Promise.all(batchJobs);
    idx += batchJobs.length;
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
    try {
      await plugGapsFromSlot(
        connection,
        nftDb,
        parserState,
        slot,
        startSeq,
        endSeq,
        treeKey,
      );
    } catch (e) {
      console.error(`Failed to plug gap from slot: ${slot}`, e);
    }
  }
}

function onlyUnique(value, index, self) {
  return self.indexOf(value) === index;
}

export async function getAllTreeSlots(
  connection: Connection,
  treeId: string,
  afterSig?: string,
  untilSig?: string,
): Promise<number[]> {
  const treeAddress = new PublicKey(treeId);
  // todo: paginate
  let lastAddress: string | null = untilSig;
  let done = false;
  const history: number[] = [];

  const baseOpts = afterSig ? { until: afterSig } : {};
  while (!done) {
    let opts = lastAddress ? { before: lastAddress } : {};
    const finalOpts = { ...baseOpts, ...opts };
    console.log(finalOpts);
    const rawSigs = (await connection.getSignaturesForAddress(treeAddress, finalOpts))
    if (rawSigs.length === 0) {
      return [];
    } else if (rawSigs.length < 1000) {
      done = true;
    }
    console.log(rawSigs);
    const sigs = rawSigs.filter((confirmedSig) => !confirmedSig.err);
    console.log(sigs);
    console.log(sigs[sigs.length - 1]);
    lastAddress = sigs[sigs.length - 1].signature;
    sigs.map((sigInfo) => {
      history.push(sigInfo.slot);
    })
  }

  return history.reverse().filter(onlyUnique);
}

/// Returns tree history in chronological order (oldest first)
/// Backfill gaps, then checks for recent transactions since gapfill
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

  // When synced up, on-chain seq # is going to be maxSeq + 1
  if (startSeq === maxSeq - 1) {
    return startSeq;
  }
  const earliestTxId = await nftDb.getTxIdForSlot(treeId, fromSlot);
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
          treeHistory[historyIndex],
          0,
          maxSeq + 1,
          treeAddress,
        )
      )
    }
    await Promise.all(batchJobs);
    numProcessed += batchJobs.length;
    console.log("num processed: ", numProcessed);
  }
  return maxSeq;
}

async function plugGapsBatched(
  batchSize: number,
  missingData,
  connection: Connection,
  nftDb: NFTDatabaseConnection,
  parserState: ParserState,
  treeId: string,
) {
  let numProcessed = 0;
  while (numProcessed < missingData.length) {
    const batchJobs = [];
    for (let i = 0; i < batchSize; i++) {
      const index = numProcessed + i;
      if (index >= missingData.length) {
        break;
      }
      batchJobs.push(
        plugGaps(
          connection,
          nftDb,
          parserState,
          treeId,
          missingData[index].prevSlot,
          missingData[index].currSlot,
          missingData[index].prevSeq,
          missingData[index].currSeq
        )
      )
    }
    numProcessed += batchJobs.length;
    await Promise.all(batchJobs);
  }

  console.log("num processed: ", numProcessed);
}

export async function fetchAndPlugGaps(
  connection: Connection,
  nftDb: NFTDatabaseConnection,
  minSeq: number,
  treeId: string,
  parserState: ParserState,
  batchSize?: number,
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

  await plugGapsBatched(
    batchSize ?? 1,
    missingData,
    connection,
    nftDb,
    parserState,
    treeId,
  );
  console.log("Done");
  return maxDbSeq;
}

async function findMissingTxSlots(
  connection: Connection,
  treeId: string,
  missingData: any[]
): Promise<BatchedGapRequest[]> {
  const mostRecentGap = missingData[missingData.length - 1];
  const txSlots = await getAllTreeSlots(
    connection,
    treeId,
    missingData[0].prevTxId,
    mostRecentGap.currTxId
  );

  const missingTxSlots = [];
  let gapIdx = 0;
  let txSlotIdx = 0;
  while (txSlotIdx < txSlots.length && gapIdx < missingData.length) {
    const slot = txSlots[txSlotIdx];
    const currGap = missingData[gapIdx];
    console.log(slot, currGap)
    if (slot > currGap.currSlot) {
      gapIdx += 1;
    } else if (slot < currGap.prevSlot) {
      // This can happen if there are too many tx's that have been returned
      txSlotIdx += 1;
      // throw new Error("tx slot is beneath current gap slot range, very likely that something is not sorted properly")
    } else {
      txSlotIdx += 1;
      missingTxSlots.push({ slot, startSeq: currGap.prevSeq, endSeq: currGap.currSeq });
    }
  }

  return missingTxSlots;
}


function calculateMissingData(missingData: GapInfo[]) {
  let missingSlots = 0;
  let missingSeqs = 0;
  for (const gap of missingData) {
    missingSlots += gap.currSlot - gap.prevSlot;
    missingSeqs += gap.currSeq - gap.prevSeq;
  }
  return { missingSlots, missingSeqs };
}

/// Fills in gaps for a given tree
/// by asychronously batching
export async function fillGapsTx(
  connection: Connection,
  db: NFTDatabaseConnection,
  parserState: ParserState,
  treeId: string,
) {
  const trees = await db.getTrees();
  const treeInfo = trees.filter((tree) => (tree[0] === treeId));
  let startSeq = 0;
  let startSlot: number | null = null;
  if (treeInfo) {
    let [missingData, maxDbSeq, maxDbSlot] = await db.getMissingDataWithTx(
      0,
      treeId
    );
    const { missingSeqs, missingSlots } = calculateMissingData(missingData);
    console.log("Missing seqs:", missingSeqs);
    console.log("Missing slots:", missingSlots);

    missingData.prevSlot
    if (missingData.length) {
      const txIdSlotPairs = await findMissingTxSlots(connection, treeId, missingData);
      console.log("Num slots to fetch:", txIdSlotPairs.length);
      await plugGapsFromSlotBatched(
        connection,
        db,
        parserState,
        treeId,
        txIdSlotPairs,
      );
    } else {
      console.log("No gaps found!")
    }
    startSlot = maxDbSlot;
    startSeq = maxDbSeq;
  }

  return { maxSeq: startSeq, maxSeqSlot: startSlot }
}

