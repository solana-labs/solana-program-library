import {
  ParsedLog,
  ParserState,
  ixRegEx,
  parseEventFromLog,
  OptionalInfo,
  dataRegEx,
  decodeEvent,
} from "./utils";
import { PROGRAM_ID as GUMMYROLL_PROGRAM_ID } from "../../gummyroll";
import { PROGRAM_ID as BUBBLEGUM_PROGRAM_ID } from "../../bubblegum/src/generated";
import { ChangeLogEvent, parseEventGummyroll } from "./gummyroll";
import {
  TokenProgramVersion,
  MetadataArgs,
} from "../../bubblegum/src/generated/types";
import { BN, Event } from "@project-serum/anchor";
import { NFTDatabaseConnection } from "../db";
import { PublicKey } from "@solana/web3.js";
import { IdlEvent } from "@project-serum/anchor/dist/cjs/idl";

function parseIxName(logLine: string): BubblegumIx | null {
  return logLine.match(ixRegEx)[1] as BubblegumIx;
}

function skipTx(sequenceNumber, startSeq, endSeq): boolean {
  let left = startSeq !== null ? sequenceNumber <= startSeq : false;
  let right = endSeq !== null ? sequenceNumber >= endSeq : false;
  return left || right;
}

export type BubblegumIx =
  | "Redeem"
  | "DecompressV1"
  | "Transfer"
  | "CreateTree"
  | "MintV1"
  | "Burn"
  | "CancelRedeem"
  | "Delegate";

export type NewLeafEvent = {
  version: TokenProgramVersion;
  metadata: MetadataArgs;
  nonce: BN;
};

export type LeafSchemaEvent = {
  schema: {
    v1: {
      id: PublicKey;
      owner: PublicKey;
      delegate: PublicKey;
      nonce: BN;
      dataHash: number[] /* size: 32 */;
      creatorHash: number[] /* size: 32 */;
    };
  };
};

export async function parseBubblegum(
  db: NFTDatabaseConnection,
  parsedLog: ParsedLog,
  slot: number,
  parser: ParserState,
  optionalInfo: OptionalInfo
) {
  const ixName = parseIxName(parsedLog.logs[0] as string);
  console.log("Bubblegum:", ixName);
  switch (ixName) {
    case "CreateTree":
      await parseBubblegumCreateTree(
        db,
        parsedLog.logs,
        slot,
        parser,
        optionalInfo
      );
      break;
    case "MintV1":
      await parseBubblegumMint(db, parsedLog.logs, slot, parser, optionalInfo);
      break;
    case "Redeem":
      await parseReplaceLeaf(
        db,
        parsedLog.logs,
        slot,
        parser,
        optionalInfo,
        false
      );
      break;
    case "CancelRedeem":
      await parseReplaceLeaf(db, parsedLog.logs, slot, parser, optionalInfo);
      break;
    case "Burn":
      await parseReplaceLeaf(db, parsedLog.logs, slot, parser, optionalInfo);
      break;
    case "Transfer":
      await parseReplaceLeaf(db, parsedLog.logs, slot, parser, optionalInfo);
      break;
    case "Delegate":
      await parseReplaceLeaf(db, parsedLog.logs, slot, parser, optionalInfo);
      break;
  }
}

function findGummyrollEvent(
  logs: (string | ParsedLog)[],
  parser: ParserState
): ChangeLogEvent | null {
  let changeLog: ChangeLogEvent | null;
  for (const log of logs) {
    if (typeof log !== "string" && log.programId.equals(GUMMYROLL_PROGRAM_ID)) {
      changeLog = parseEventGummyroll(log, parser.Gummyroll);
    }
  }
  if (!changeLog) {
    console.log("Failed to find gummyroll changelog");
  }
  return changeLog;
}

function findBubblegumEvents(
  logs: (string | ParsedLog)[],
  parser: ParserState
): Array<Event> {
  let events = [];
  for (const log of logs) {
    if (typeof log !== "string") {
      continue;
    }
    let data = log.match(dataRegEx);
    if (data && data.length > 1) {
      events.push(decodeEvent(data[1], parser.Bubblegum.idl));
    }
  }
  return events;
}

export async function parseBubblegumMint(
  db: NFTDatabaseConnection,
  logs: (string | ParsedLog)[],
  slot: number,
  parser: ParserState,
  optionalInfo: OptionalInfo
) {
  const changeLog = findGummyrollEvent(logs, parser);
  const events = findBubblegumEvents(logs, parser);
  if (events.length !== 2) {
    return;
  }
  const newLeafData = events[0].data as NewLeafEvent;
  const leafSchema = events[1].data as LeafSchemaEvent;
  let treeId = changeLog.id.toBase58();
  let sequenceNumber = changeLog.seq;
  let { startSeq, endSeq, txId } = optionalInfo;
  if (skipTx(sequenceNumber, startSeq, endSeq)) {
    return;
  }
  console.log(`Sequence Number: ${sequenceNumber}`);
  await db.updateNFTMetadata(newLeafData, leafSchema.schema.v1.id.toBase58());
  await db.updateLeafSchema(
    leafSchema,
    new PublicKey(changeLog.path[0].node),
    txId,
    slot,
    sequenceNumber,
    treeId
  );
  await db.updateChangeLogs(changeLog, optionalInfo.txId, slot, treeId);
}

export async function parseReplaceLeaf(
  db: NFTDatabaseConnection,
  logs: (string | ParsedLog)[],
  slot: number,
  parser: ParserState,
  optionalInfo: OptionalInfo,
  compressed: boolean = true
) {
  const changeLog = findGummyrollEvent(logs, parser);
  const events = findBubblegumEvents(logs, parser);
  if (events.length !== 1) {
    return;
  }
  const leafSchema = events[0].data as LeafSchemaEvent;
  let treeId = changeLog.id.toBase58();
  let sequenceNumber = changeLog.seq;
  let { startSeq, endSeq, txId } = optionalInfo;
  if (skipTx(sequenceNumber, startSeq, endSeq)) {
    return;
  }
  console.log(`Sequence Number: ${sequenceNumber}`);
  await db.updateLeafSchema(
    leafSchema,
    new PublicKey(changeLog.path[0].node),
    txId,
    slot,
    sequenceNumber,
    treeId,
    compressed
  );
  await db.updateChangeLogs(changeLog, optionalInfo.txId, slot, treeId);
}

export async function parseBubblegumCreateTree(
  db: NFTDatabaseConnection,
  logs: (string | ParsedLog)[],
  slot: number,
  parser: ParserState,
  optionalInfo: OptionalInfo
) {
  const changeLog = findGummyrollEvent(logs, parser);
  const sequenceNumber = changeLog.seq;
  let { startSeq, endSeq, txId } = optionalInfo;
  if (skipTx(sequenceNumber, startSeq, endSeq)) {
    return;
  }
  console.log(`Sequence Number: ${sequenceNumber}`);
  let treeId = changeLog.id.toBase58();
  await db.updateChangeLogs(changeLog, optionalInfo.txId, slot, treeId);
}

export async function parseBubblegumDecompress(
  db: NFTDatabaseConnection,
  logs: (string | ParsedLog)[],
  parser: ParserState,
  optionalInfo: OptionalInfo
) {}
