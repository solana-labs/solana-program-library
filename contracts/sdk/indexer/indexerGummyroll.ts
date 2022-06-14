import { BN, web3 } from "@project-serum/anchor";
import { PublicKey } from "@solana/web3.js";
import React from "react";
import { emptyNode, hash } from "../../tests/merkle-tree";
import { PathNode, decodeMerkleRoll, OnChainMerkleRoll } from "../gummyroll";
import { NFTDatabaseConnection } from "./db";

export async function updateMerkleRollSnapshot(
  connection: web3.Connection,
  merkleRollKey: PublicKey,
  setMerkleRoll: any
) {
  const result = await connection.getAccountInfo(merkleRollKey, "confirmed");
  if (result) {
    setMerkleRoll(decodeMerkleRoll(result?.data));
  }
}

export async function updateMerkleRollLive(
  connection: web3.Connection,
  merkleRollKey: PublicKey,
  setMerkleRoll: any
) {
  let subId = connection.onAccountChange(
    merkleRollKey,
    (result) => {
      if (result) {
        try {
          setMerkleRoll(decodeMerkleRoll(result?.data));
        } catch (e) {
          console.log("Failed to deserialize account", e);
        }
      }
    },
    "confirmed"
  );
  return subId;
}

export async function getUpdatedBatch(
  merkleRoll: OnChainMerkleRoll,
  db: NFTDatabaseConnection
) {
  const seq = merkleRoll.roll.sequenceNumber.toNumber();
  let rows: Array<[PathNode, number, number]> = [];
  if (seq === 0) {
    let nodeIdx = 1 << merkleRoll.header.maxDepth;
    for (let i = 0; i < merkleRoll.header.maxDepth; ++i) {
      rows.push([
        {
          node: new PublicKey(db.emptyNode(i)),
          index: nodeIdx,
        },
        0,
        i,
      ]);
      nodeIdx >>= 1;
    }
    rows.push([
      {
        node: new PublicKey(db.emptyNode(merkleRoll.header.maxDepth)),
        index: 1,
      },
      0,
      merkleRoll.header.maxDepth,
    ]);
  } else {
    const pathNodes = merkleRoll.getChangeLogsWithNodeIndex();
    console.log(`Received Batch! Sequence=${seq}, entries ${pathNodes.length}`);
    let data: Array<[number, PathNode[]]> = [];
    for (const [i, path] of pathNodes.entries()) {
      if (i == seq) {
        break;
      }
      data.push([seq - i, path]);
    }

    let sequenceNumbers = await db.getSequenceNumbers();
    for (const [seq, path] of data) {
      if (sequenceNumbers.has(seq)) {
        continue;
      }
      for (const [i, node] of path.entries()) {
        rows.push([node, seq, i]);
      }
    }
  }
  db.upsertRowsFromBackfill(rows);
  console.log(`Updated ${rows.length} rows`);
  await db.updateTree();
}
