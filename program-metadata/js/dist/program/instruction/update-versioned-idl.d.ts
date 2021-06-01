/// <reference types="node" />
import { Struct } from "../util/borsh-struct";
import { SerializationMethod } from "./create-versioned-idl";
export declare class UpdateVersionedIdlInstruction extends Struct {
    idlUrl: string;
    idlHash: Buffer;
    sourceUrl: string;
    customLayoutUrl: null | string;
    instruction: number;
    serialization: any;
    constructor(idlUrl: string, idlHash: Buffer, sourceUrl: string, serialization: SerializationMethod, customLayoutUrl: null | string);
}
