import * as anchor from '@project-serum/anchor';
import { PublicKey } from '@solana/web3.js';
import { Gummyroll, PathNode } from "../../gummyroll"
import { parseEventFromLog, ParsedLog, ixRegEx } from './utils';

export type GummyrollIx =
    'InitEmptyGummyroll' | 'InitEmptyGummyrollWithRoot' | 'Replace' | 'Append' | 'InsertOrAppend' | 'VerifyLeaf' | 'TransferAuthority';

export type ChangeLogEvent = {
    id: PublicKey,
    path: PathNode[],
    seq: number,
    index: number,
};

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
