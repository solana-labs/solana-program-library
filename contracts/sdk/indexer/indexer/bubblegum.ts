import {
  ParsedLog,
  ParserState,
  ixRegEx,
  parseEventFromLog,
  OptionalInfo,
} from "./utils";
import { PROGRAM_ID as GUMMYROLL_PROGRAM_ID } from "../../gummyroll";
import { ChangeLogEvent, parseEventGummyroll } from "./gummyroll";
import {
  LeafSchema,
  TokenProgramVersion,
  MetadataArgs,
} from "../../bubblegum/src/generated/types";
import { BN } from "@project-serum/anchor";
import { NFTDatabaseConnection } from "../db";
import { PublicKey } from "@solana/web3.js";

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
  | "Decompress"
  | "Transfer"
  | "CreateTree"
  | "Mint"
  | "CancelRedeem"
  | "Delegate";

export type NewLeafEvent = {
  version: TokenProgramVersion;
  metadata: MetadataArgs;
  nonce: BN;
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
    case "Mint":
      await parseBubblegumMint(db, parsedLog.logs, slot, parser, optionalInfo);
      break;
    case "Redeem":
      await parseBubblegumRedeem(
        db,
        parsedLog.logs,
        slot,
        parser,
        optionalInfo
      );
      break;
    case "CancelRedeem":
      await parseBubblegumCancelRedeem(
        db,
        parsedLog.logs,
        slot,
        parser,
        optionalInfo
      );
      break;
    case "Transfer":
      await parseBubblegumTransfer(
        db,
        parsedLog.logs,
        slot,
        parser,
        optionalInfo
      );
      break;
    case "Delegate":
      await parseBubblegumDelegate(
        db,
        parsedLog.logs,
        slot,
        parser,
        optionalInfo
      );
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

export async function parseBubblegumMint(
  db: NFTDatabaseConnection,
  logs: (string | ParsedLog)[],
  slot: number,
  parser: ParserState,
  optionalInfo: OptionalInfo
) {
  const changeLog = findGummyrollEvent(logs, parser);
  const newLeafData = parseEventFromLog(logs[1] as string, parser.Bubblegum.idl)
    .data as NewLeafEvent;
  const leafSchema = parseEventFromLog(logs[2] as string, parser.Bubblegum.idl)
    .data as LeafSchema;
  let treeId = changeLog.id.toBase58();
  let sequenceNumber = changeLog.seq;
  let { startSeq, endSeq, txId } = optionalInfo;
  if (skipTx(sequenceNumber, startSeq, endSeq)) {
    return;
  }
  console.log(`Sequence Number: ${sequenceNumber}`);
  await db.updateNFTMetadata(newLeafData, leafSchema.nonce, treeId);
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

export async function parseBubblegumTransfer(
  db: NFTDatabaseConnection,
  logs: (string | ParsedLog)[],
  slot: number,
  parser: ParserState,
  optionalInfo: OptionalInfo
) {
  const changeLog = findGummyrollEvent(logs, parser);
  const leafSchema = parseEventFromLog(logs[1] as string, parser.Bubblegum.idl)
    .data as LeafSchema;
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
    treeId
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
  let treeId = changeLog.id.toBase58();
  await db.updateChangeLogs(changeLog, optionalInfo.txId, slot, treeId);
}

export async function parseBubblegumDelegate(
  db: NFTDatabaseConnection,
  logs: (string | ParsedLog)[],
  slot: number,
  parser: ParserState,
  optionalInfo: OptionalInfo
) {
  const changeLog = findGummyrollEvent(logs, parser);
  const leafSchema = parseEventFromLog(logs[1] as string, parser.Bubblegum.idl)
    .data as LeafSchema;
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
    treeId
  );
  await db.updateChangeLogs(changeLog, optionalInfo.txId, slot, treeId);
}

export async function parseBubblegumRedeem(
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
  await db.updateChangeLogs(changeLog, txId, slot, treeId);
}

export async function parseBubblegumCancelRedeem(
  db: NFTDatabaseConnection,
  logs: (string | ParsedLog)[],
  slot: number,
  parser: ParserState,
  optionalInfo: OptionalInfo
) {
  const changeLog = findGummyrollEvent(logs, parser);
  const leafSchema = parseEventFromLog(logs[1] as string, parser.Bubblegum.idl)
    .data as LeafSchema;
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
    treeId
  );
  await db.updateChangeLogs(changeLog, optionalInfo.txId, slot, treeId);
}

export async function parseBubblegumDecompress(
  db: NFTDatabaseConnection,
  logs: (string | ParsedLog)[],
  parser: ParserState,
  optionalInfo: OptionalInfo
) {}
