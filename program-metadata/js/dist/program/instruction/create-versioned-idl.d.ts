/// <reference types="node" />
import { Struct } from "../util/borsh-struct";
export declare class CreateVersionedIdlInstruction extends Struct {
    effectiveSlot: number;
    idlUrl: string;
    idlHash: Buffer;
    sourceUrl: string;
    hashedName: Buffer;
    instruction: number;
    constructor(effectiveSlot: number, idlUrl: string, idlHash: Buffer, sourceUrl: string, hashedName: Buffer);
}
