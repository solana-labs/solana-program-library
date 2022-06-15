import * as anchor from "@project-serum/anchor";
import { PublicKey } from "@solana/web3.js";
import { readFileSync } from "fs";
import { Bubblegum } from "../../../target/types/bubblegum";
import { Gummyroll } from "../../../target/types/gummyroll";

const startRegEx = /Program (\w*) invoke \[(\d)\]/;
const endRegEx = /Program (\w*) success/;
const dataRegEx =
  /Program data: ((?:[A-Za-z\d+/]{4})*(?:[A-Za-z\d+/]{3}=|[A-Za-z\d+/]{2}==)?$)/;
export const ixRegEx = /Program log: Instruction: (\w+)/;

export type ParserState = {
  Gummyroll: anchor.Program<Gummyroll>;
  Bubblegum: anchor.Program<Bubblegum>;
};

export type ParsedLog = {
  programId: PublicKey;
  logs: (string | ParsedLog)[];
  depth: number;
};

export type OptionalInfo = {
  txId: string;
};

/**
 * Recursively parses the logs of a program instruction execution
 * @param programId
 * @param depth
 * @param logs
 * @returns
 */
function parseInstructionLog(
  programId: PublicKey,
  depth: number,
  logs: string[]
) {
  const parsedLog: ParsedLog = {
    programId,
    depth,
    logs: [],
  };
  let instructionComplete = false;
  while (!instructionComplete) {
    const logLine = logs[0];
    logs = logs.slice(1);
    let result = logLine.match(endRegEx);
    if (result) {
      if (result[1] != programId.toString()) {
        throw Error(`Unexpected program id finished: ${result[1]}`);
      }
      instructionComplete = true;
    } else {
      result = logLine.match(startRegEx);
      if (result) {
        const programId = new PublicKey(result[1]);
        const depth = Number(result[2]) - 1;
        const parsedInfo = parseInstructionLog(programId, depth, logs);
        parsedLog.logs.push(parsedInfo.parsedLog);
        logs = parsedInfo.logs;
      } else {
        parsedLog.logs.push(logLine);
      }
    }
  }
  return { parsedLog, logs };
}

/**
 * Parses logs so that emitted event data can be tied to its execution context
 * @param logs
 * @returns
 */
export function parseLogs(logs: string[]): ParsedLog[] {
  let parsedLogs: ParsedLog[] = [];
  while (logs && logs.length) {
    const logLine = logs[0];
    logs = logs.slice(1);
    const result = logLine.match(startRegEx);
    const programId = new PublicKey(result[1]);
    const depth = Number(result[2]) - 1;
    const parsedInfo = parseInstructionLog(programId, depth, logs);
    parsedLogs.push(parsedInfo.parsedLog);
    logs = parsedInfo.logs;
  }
  return parsedLogs;
}

export function parseEventFromLog(
  log: string,
  idl: anchor.Idl
): anchor.Event | null {
  return decodeEvent(log.match(dataRegEx)[1], idl);
}

/**
 * Example:
 * ```
 * let event = decodeEvent(dataString, Gummyroll.idl) ?? decodeEvent(dataString, Bubblegum.idl);
 * ```
 * @param data
 * @param idl
 * @returns
 */
function decodeEvent(data: string, idl: anchor.Idl): anchor.Event | null {
  let eventCoder = new anchor.BorshEventCoder(idl);
  return eventCoder.decode(data);
}

export function loadProgram(
  provider: anchor.Provider,
  programId: PublicKey,
  idlPath: string
) {
  const IDL = JSON.parse(readFileSync(idlPath).toString());
  return new anchor.Program(IDL, programId, provider);
}
