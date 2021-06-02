import { TransactionInstruction } from "@solana/web3.js";
import { Coder, DecodedInstruction } from "../coder";
export declare class Borsh extends Coder {
    decodeInstruction(instruction: TransactionInstruction): DecodedInstruction;
    private getInstruction;
    private getInstructionIndex;
    private getAccounts;
    private getArguments;
    private buildCoder;
}
