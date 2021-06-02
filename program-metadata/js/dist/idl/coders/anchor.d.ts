import { TransactionInstruction } from "@solana/web3.js";
import { Coder, DecodedInstruction } from "../coder";
export declare class Anchor extends Coder {
    decodeInstruction(instruction: TransactionInstruction): DecodedInstruction;
}
