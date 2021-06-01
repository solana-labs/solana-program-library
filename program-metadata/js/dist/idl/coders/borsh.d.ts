import { TransactionInstruction } from "@solana/web3.js";
import { Coder, DecodedInstruction } from "../coder";
export declare class Borsh extends Coder {
    private getInstruction;
    private getInstructionIndex;
    private getAccounts;
    private getArguments;
    private buildSchema;
    private buildDataObject;
    decodeInstruction(instruction: TransactionInstruction): DecodedInstruction;
}
