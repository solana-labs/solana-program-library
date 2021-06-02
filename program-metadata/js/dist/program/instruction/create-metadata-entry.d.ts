/// <reference types="node" />
import { Struct } from "../util/borsh-struct";
export declare class CreateMetadataEntryInstruction extends Struct {
    name: string;
    value: string;
    hashedName: Buffer;
    instruction: number;
    constructor(name: string, value: string, hashedName: Buffer);
}
