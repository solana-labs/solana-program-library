import { Idl } from "@project-serum/anchor";
import { IdlAccountItem, IdlType } from "./idl";
import { PublicKey, TransactionInstruction } from "@solana/web3.js";
export declare abstract class Coder {
    protected idl: Idl;
    constructor(idl: Idl);
    abstract decodeInstruction(instruction: TransactionInstruction): DecodedInstruction;
}
export interface DecodedInstruction {
    name: string;
    formattedName: string;
    programId: PublicKey;
    accounts: DecodedAccount[];
    args: any[];
}
export declare type DecodedAccount = {
    formattedName: string;
    pubkey: PublicKey;
} & IdlAccountItem | AccountError;
export declare type AccountError = {
    message: string;
};
export declare type DecodedArgument = {
    name: string;
    formattedName: string;
    value: any;
    type: IdlType;
};
