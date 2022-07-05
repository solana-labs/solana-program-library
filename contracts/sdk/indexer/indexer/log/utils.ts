import { PublicKey } from '@solana/web3.js';
import * as anchor from '@project-serum/anchor';
import { decodeEvent } from '../utils';

export const startRegEx = /Program (\w*) invoke \[(\d)\]/;
export const endRegEx = /Program (\w*) success/;
export const dataRegEx =
    /Program data: ((?:[A-Za-z\d+/]{4})*(?:[A-Za-z\d+/]{3}=|[A-Za-z\d+/]{2}==)?$)/;
export const ixRegEx = /Program log: Instruction: (\w+)/;

export function parseEventFromLog(
    log: string,
    idl: anchor.Idl
): anchor.Event | null {
    return decodeEvent(log.match(dataRegEx)[1], idl);
}

export type ParsedLog = {
    programId: PublicKey;
    logs: (string | ParsedLog)[];
    depth: number;
};
