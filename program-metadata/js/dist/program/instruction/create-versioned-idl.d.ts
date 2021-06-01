/// <reference types="node" />
import { SerializationMethod } from "../../idl/idl-coder";
import { Struct } from "../util/borsh-struct";
export declare class CreateVersionedIdlInstruction extends Struct {
    effectiveSlot: number;
    idlUrl: string;
    idlHash: Buffer;
    sourceUrl: string;
    customLayoutUrl: null | string;
    hashedName: Buffer;
    instruction: number;
    serialization: any;
    constructor(effectiveSlot: number, idlUrl: string, idlHash: Buffer, sourceUrl: string, serialization: SerializationMethod, customLayoutUrl: null | string, hashedName: Buffer);
}
