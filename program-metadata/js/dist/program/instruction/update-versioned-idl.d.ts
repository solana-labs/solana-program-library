/// <reference types="node" />
import { Struct } from "../util/borsh-struct";
export declare class UpdateVersionedIdlInstruction extends Struct {
    idlUrl: string;
    idlHash: Buffer;
    sourceUrl: string;
    instruction: number;
    constructor(idlUrl: string, idlHash: Buffer, sourceUrl: string);
}
