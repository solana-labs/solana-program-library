import { PublicKey, Keypair, TransactionInstruction, SystemProgram, Connection } from "@solana/web3.js";
import { PROGRAM_ID } from ".";
import { getMerkleRollAccountSize } from "./accounts";
import * as anchor from "@project-serum/anchor";
import { Gummyroll } from "./types";
import { CANDY_WRAPPER_PROGRAM_ID } from "@sorend-solana/utils";

export async function createAllocTreeIx(
    connection: Connection,
    maxBufferSize: number,
    maxDepth: number,
    canopyDepth: number,
    payer: PublicKey,
    merkleRoll: PublicKey,
): Promise<TransactionInstruction> {
    const requiredSpace = getMerkleRollAccountSize(
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

export async function getCreateTreeIxs(
    gummyroll: anchor.Program<Gummyroll>,
    maxBufferSize: number,
    maxDepth: number,
    canopyDepth: number,
    payer: PublicKey,
    merkleRoll: PublicKey,
    authority: Keypair,
    appendAuthority: PublicKey,
): Promise<TransactionInstruction[]> {
    const allocAccountIx = await createAllocTreeIx(
        gummyroll.provider.connection,
        maxBufferSize,
        maxDepth,
        canopyDepth,
        payer,
        merkleRoll,
    );
    const initIx = gummyroll.instruction.initEmptyGummyroll(
        maxDepth,
        maxBufferSize,
        {
            accounts: {
                merkleRoll,
                authority: authority.publicKey,
                candyWrapper: CANDY_WRAPPER_PROGRAM_ID
            },
            signers: [authority],
        },
    )

    return [allocAccountIx, initIx];
}
