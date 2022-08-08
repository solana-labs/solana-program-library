import { PublicKey, Logs, Context } from "@solana/web3.js";
import * as anchor from '@project-serum/anchor';
import { NFTDatabaseConnection } from "../../db";
import { ParserState, ParseResult, OptionalInfo, decodeEvent } from "../utils";
import {
    BubblegumIx,
    LeafSchemaEvent,
    NewLeafEvent,
    ingestBubblegumCreateTree,
    ingestBubblegumMint,
    ingestBubblegumReplaceLeaf,
    ingestBubblegumDecompressLeaf,
    NFTDecompressionEvent
} from "../ingester";
import { ParsedLog, endRegEx, startRegEx, ixRegEx, dataRegEx } from './utils';
import { PROGRAM_ID as BUBBLEGUM_PROGRAM_ID } from "@sorend-solana/bubblegum";
import { findGummyrollEvent } from './gummyroll';

function findBubblegumEvents(
    logs: (string | ParsedLog)[],
    parser: ParserState
): Array<anchor.Event> {
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


/**
 * Performs a depth-first traversal of the ParsedLog data structure
 * @param db
 * @param optionalInfo
 * @param slot
 * @param parsedState
 * @param parsedLog
 * @returns
 */
async function indexParsedLog(
    db: NFTDatabaseConnection,
    optionalInfo: OptionalInfo,
    slot: number,
    parserState: ParserState,
    parsedLog: ParsedLog | string
) {
    if (typeof parsedLog === "string") {
        return;
    }
    if (parsedLog.programId.equals(BUBBLEGUM_PROGRAM_ID)) {
        return await parseBubblegumLog(db, parsedLog, slot, parserState, optionalInfo);
    } else {
        for (const log of parsedLog.logs) {
            await indexParsedLog(db, optionalInfo, slot, parserState, log);
        }
    }
}

function isLogTruncated(logs: string[]): boolean {
    // Loops backward
    for (let i = logs.length - 1; i >= 0; i--) {
        if (logs[i].startsWith("Log truncated")) {
            return true;
        }
    }
    return false;
}

/**
 * Returns false if log is truncated
 */
export function handleLogsAtomic(
    db: NFTDatabaseConnection,
    logs: Logs,
    context: Context,
    parsedState: ParserState,
    startSeq: number | null = null,
    endSeq: number | null = null
): ParseResult {
    if (logs.err) {
        return ParseResult.TransactionError;
    }
    if (isLogTruncated(logs.logs)) {
        return ParseResult.LogTruncated;
    }
    const parsedLogs = parseLogs(logs.logs);
    if (parsedLogs.length === 0) {
        return ParseResult.Success;
    }
    db.connection.db.serialize(() => {
        db.beginTransaction();
        for (const parsedLog of parsedLogs) {
            indexParsedLog(
                db,
                { txId: logs.signature, startSeq, endSeq },
                context.slot,
                parsedState,
                parsedLog
            );
        }
        db.commit();
    });
    return ParseResult.Success;
}

/**
 * Processes the logs from a new transaction and searches for the programs
 * specified in the ParserState
 * @param db
 * @param logs
 * @param context
 * @param parsedState
 * @param startSeq
 * @param endSeq
 * @returns
 */
export async function handleLogs(
    db: NFTDatabaseConnection,
    logs: Logs,
    context: Context,
    parsedState: ParserState,
    startSeq: number | null = null,
    endSeq: number | null = null
) {
    if (logs.err) {
        return;
    }
    const parsedLogs = parseLogs(logs.logs);
    if (parsedLogs.length == 0) {
        return;
    }
    for (const parsedLog of parsedLogs) {
        await indexParsedLog(
            db,
            { txId: logs.signature, startSeq, endSeq },
            context.slot,
            parsedState,
            parsedLog
        );
    }
}

function parseIxName(logLine: string): BubblegumIx | null {
    return logLine.match(ixRegEx)[1] as BubblegumIx;
}

export async function parseBubblegumLog(
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
        case "DecompressV1":
            await parseBubblegumDecompressLeaf(
                db,
                parsedLog.logs,
                slot,
                parser,
                optionalInfo,
            )
            break;
        case "Redeem":
            await parseBubblegumReplaceLeaf(
                db,
                parsedLog.logs,
                slot,
                parser,
                optionalInfo,
                true,
                true
            );
            break;
        case "CancelRedeem":
            await parseBubblegumReplaceLeaf(db, parsedLog.logs, slot, parser, optionalInfo, false, true);
            break;
        case "Burn":
            await parseBubblegumReplaceLeaf(db, parsedLog.logs, slot, parser, optionalInfo);
            break;
        case "Transfer":
            await parseBubblegumReplaceLeaf(db, parsedLog.logs, slot, parser, optionalInfo);
            break;
        case "Delegate":
            await parseBubblegumReplaceLeaf(db, parsedLog.logs, slot, parser, optionalInfo);
            break;
    }
}

async function parseBubblegumCreateTree(
    db: NFTDatabaseConnection,
    logs: (string | ParsedLog)[],
    slot: number,
    parser: ParserState,
    optionalInfo: OptionalInfo
) {
    const changeLog = findGummyrollEvent(logs, parser);
    await ingestBubblegumCreateTree(db, slot, optionalInfo, changeLog);
}

async function parseBubblegumMint(
    db: NFTDatabaseConnection,
    logs: (string | ParsedLog)[],
    slot: number,
    parser: ParserState,
    optionalInfo: OptionalInfo
) {
    const changeLog = findGummyrollEvent(logs, parser);
    const events = findBubblegumEvents(logs, parser);
    if (events.length !== 2) {
        console.error("Could not find enough Bubblegum events for Bubblegum mint");
        return;
    }
    const newLeafData = events[0].data as NewLeafEvent;
    const leafSchema = events[1].data as LeafSchemaEvent;
    await ingestBubblegumMint(
        db,
        slot,
        optionalInfo,
        changeLog,
        newLeafData,
        leafSchema
    )
}

async function parseBubblegumReplaceLeaf(
    db: NFTDatabaseConnection,
    logs: (string | ParsedLog)[],
    slot: number,
    parser: ParserState,
    optionalInfo: OptionalInfo,
    redeemed: boolean = false,
    compressed: boolean = true
) {
    const changeLog = findGummyrollEvent(logs, parser);
    const events = findBubblegumEvents(logs, parser);
    if (events.length !== 1) {
        console.error("Could not find leafSchema event for Bubblegum replace");
        return;
    }
    const leafSchema = events[0].data as LeafSchemaEvent;
    await ingestBubblegumReplaceLeaf(
        db,
        slot,
        optionalInfo,
        changeLog,
        leafSchema,
        redeemed,
        compressed
    )
}

async function parseBubblegumDecompressLeaf(
    db: NFTDatabaseConnection,
    logs: (string | ParsedLog)[],
    slot: number,
    parser: ParserState,
    optionalInfo: OptionalInfo,
    redeemed: boolean = false,
    compressed: boolean = true
) {
    const events = findBubblegumEvents(logs, parser);
    if (events.length !== 1) {
        console.error("Could not find leafSchema event for Bubblegum decompress");
        return;
    }
    const decompressionEvent = events[0].data as NFTDecompressionEvent;
    console.log(decompressionEvent)
    await ingestBubblegumDecompressLeaf(
        db,
        decompressionEvent
    )
}
