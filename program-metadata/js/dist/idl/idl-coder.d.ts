import { TransactionInstruction } from "@solana/web3.js";
import { Coder, DecodedInstruction } from "./coder";
import { Idl, SerializationMethod } from "./idl";
export declare const CODER_MAP: Map<SerializationMethod, new (idl: Idl) => Coder>;
export declare class IdlCoder {
    private idl;
    private coder;
    constructor(idl: Idl);
    decodeInstruction(instruction: TransactionInstruction): DecodedInstruction;
}
