import * as anchor from '@project-serum/anchor';
import { PublicKey } from '@solana/web3.js';
import { Gummyroll } from "../../../../target/types/gummyroll";
import { PROGRAM_ID as GUMMYROLL_PROGRAM_ID } from "@sorend-solana/gummyroll";
import { parseEventFromLog, ParsedLog, ixRegEx } from './utils';
import { ParserState } from '../utils';
import { ChangeLogEvent } from '../ingester';

export type GummyrollIx =
    'InitEmptyGummyroll' | 'InitEmptyGummyrollWithRoot' | 'Replace' | 'Append' | 'InsertOrAppend' | 'VerifyLeaf' | 'TransferAuthority';

function parseIxName(logLine: string): GummyrollIx | null {
    return logLine.match(ixRegEx)[1] as GummyrollIx
}

/// Returns a changelog or null
export function parseEventGummyroll(parsedLog: ParsedLog, gummyroll: anchor.Program<Gummyroll>): ChangeLogEvent | null {
    const ixName = parseIxName(parsedLog.logs[0] as string);
    console.log("\tGummyroll:", ixName);
    switch (ixName) {
        case 'VerifyLeaf':
        case 'TransferAuthority':
            console.log("Skipping")
            return null;
        default:
            return parseChangelogEvent(parsedLog.logs as string[], gummyroll);
    }
}

function parseChangelogEvent(logs: string[], gummyroll: anchor.Program<Gummyroll>): ChangeLogEvent | null {
    return parseEventFromLog(logs[logs.length - 2], gummyroll.idl).data as ChangeLogEvent;
}

export function findGummyrollEvent(
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
