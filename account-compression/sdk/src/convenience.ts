import { PublicKey, TransactionInstruction, SystemProgram, Connection } from "@solana/web3.js";
import { PROGRAM_ID } from "./generated";
import { getConcurrentMerkleTreeAccountSize } from "./accounts";

export async function createAllocTreeIx(
    connection: Connection,
    maxBufferSize: number,
    maxDepth: number,
    canopyDepth: number,
    payer: PublicKey,
    merkleRoll: PublicKey,
): Promise<TransactionInstruction> {
    const requiredSpace = getConcurrentMerkleTreeAccountSize(
        maxDepth,
        maxBufferSize,
        canopyDepth ?? 0
    );
    return SystemProgram.createAccount({
        fromPubkey: payer,
        newAccountPubkey: merkleRoll,
        lamports:
            await connection.getMinimumBalanceForRentExemption(
                requiredSpace
            ),
        space: requiredSpace,
        programId: PROGRAM_ID
    });
}
