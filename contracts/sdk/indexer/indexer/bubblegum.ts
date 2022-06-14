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

export function parseBubblegum(
  db: NFTDatabaseConnection,
  parsedLog: ParsedLog,
  parser: ParserState,
  optionalInfo: OptionalInfo
) {
  const ixName = parseIxName(parsedLog.logs[0] as string);
  console.log("Bubblegum:", ixName);
  switch (ixName) {
    case "CreateTree":
      parseBubblegumCreateTree(db, parsedLog.logs, parser, optionalInfo);
      break;
    case "Mint":
      parseBubblegumMint(db, parsedLog.logs, parser, optionalInfo);
      break;
    case "Redeem":
      parseBubblegumRedeem(db, parsedLog.logs, parser, optionalInfo);
      break;
    case "CancelRedeem":
      parseBubblegumCancelRedeem(db, parsedLog.logs, parser, optionalInfo);
      break;
    case "Transfer":
      parseBubblegumTransfer(db, parsedLog.logs, parser, optionalInfo);
      break;
    case "Delegate":
      parseBubblegumDelegate(db, parsedLog.logs, parser, optionalInfo);
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

export function parseBubblegumMint(
  db: NFTDatabaseConnection,
  logs: (string | ParsedLog)[],
  parser: ParserState,
  optionalInfo: OptionalInfo
) {
  const changeLog = findGummyrollEvent(logs, parser);
  const newLeafData = parseEventFromLog(logs[1] as string, parser.Bubblegum.idl)
    .data as NewLeafEvent;
  const leafSchema = parseEventFromLog(logs[2] as string, parser.Bubblegum.idl)
    .data as LeafSchema;
  db.updateNFTMetadata(newLeafData, leafSchema.nonce);
  db.updateLeafSchema(
    leafSchema,
    new PublicKey(changeLog.path[0].node),
    optionalInfo.txId
  );
  db.updateChangeLogs(changeLog, optionalInfo.txId);
}

export function parseBubblegumTransfer(
  db: NFTDatabaseConnection,
  logs: (string | ParsedLog)[],
  parser: ParserState,
  optionalInfo: OptionalInfo
) {
  const changeLog = findGummyrollEvent(logs, parser);
  const leafSchema = parseEventFromLog(logs[1] as string, parser.Bubblegum.idl)
    .data as LeafSchema;
  db.updateLeafSchema(
    leafSchema,
    new PublicKey(changeLog.path[0].node),
    optionalInfo.txId
  );
  db.updateChangeLogs(changeLog, optionalInfo.txId);
}

export function parseBubblegumCreateTree(
  db: NFTDatabaseConnection,
  logs: (string | ParsedLog)[],
  parser: ParserState,
  optionalInfo: OptionalInfo
) {
  const changeLog = findGummyrollEvent(logs, parser);
  db.updateChangeLogs(changeLog, optionalInfo.txId);
}

export function parseBubblegumDelegate(
  db: NFTDatabaseConnection,
  logs: (string | ParsedLog)[],
  parser: ParserState,
  optionalInfo: OptionalInfo
) {
  const changeLog = findGummyrollEvent(logs, parser);
  const leafSchema = parseEventFromLog(logs[1] as string, parser.Bubblegum.idl)
    .data as LeafSchema;
  db.updateLeafSchema(
    leafSchema,
    new PublicKey(changeLog.path[0].node),
    optionalInfo.txId
  );
  db.updateChangeLogs(changeLog, optionalInfo.txId);
}

export function parseBubblegumRedeem(
  db: NFTDatabaseConnection,
  logs: (string | ParsedLog)[],
  parser: ParserState,
  optionalInfo: OptionalInfo
) {
  const changeLog = findGummyrollEvent(logs, parser);
  db.updateChangeLogs(changeLog, optionalInfo.txId);
}

export function parseBubblegumCancelRedeem(
  db: NFTDatabaseConnection,
  logs: (string | ParsedLog)[],
  parser: ParserState,
  optionalInfo: OptionalInfo
) {
  const changeLog = findGummyrollEvent(logs, parser);
  const leafSchema = parseEventFromLog(logs[1] as string, parser.Bubblegum.idl)
    .data as LeafSchema;
  db.updateLeafSchema(
    leafSchema,
    new PublicKey(changeLog.path[0].node),
    optionalInfo.txId
  );
  db.updateChangeLogs(changeLog, optionalInfo.txId);
}

export function parseBubblegumDecompress(
  db: NFTDatabaseConnection,
  logs: (string | ParsedLog)[],
  parser: ParserState,
  optionalInfo: OptionalInfo
) {}
