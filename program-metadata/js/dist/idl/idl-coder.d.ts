import { TransactionInstruction } from "@solana/web3.js";
import { Coder } from "./coder";
import { Idl } from "./idl";
export declare enum SerializationMethod {
    Bincode = 0,
    Borsh = 1,
    Anchor = 2
}
export declare const CODER_MAP: Map<SerializationMethod, new (idl: Idl) => Coder>;
export declare class IdlCoder {
    private idl;
    private serializationMethod;
    private coder;
    constructor(idl: Idl, serializationMethod: SerializationMethod);
    decodeInstruction(instruction: TransactionInstruction): import("./coder").DecodedInstruction;
    decodeAccount(account: any): void;
}
