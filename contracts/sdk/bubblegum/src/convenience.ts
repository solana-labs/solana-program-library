import { BN } from "@project-serum/anchor";
import { TransactionInstruction, PublicKey, Connection, AccountInfo } from "@solana/web3.js";
import { Nonce, PROGRAM_ID } from './generated';
import { CANDY_WRAPPER_PROGRAM_ID } from "../../utils";
import { PROGRAM_ID as GUMMYROLL_PROGRAM_ID, createAllocTreeIx } from "../../gummyroll";
import { createCreateTreeInstruction } from "./generated";

export async function getBubblegumAuthorityPDA(merkleRollPubKey: PublicKey) {
    const [bubblegumAuthorityPDAKey] = await PublicKey.findProgramAddress(
        [merkleRollPubKey.toBuffer()],
        PROGRAM_ID
    );
    return bubblegumAuthorityPDAKey;
}

export async function getNonceCount(connection: Connection, tree: PublicKey): Promise<BN> {
    const treeAuthority = await getBubblegumAuthorityPDA(tree);
    return new BN((await Nonce.fromAccountAddress(connection, treeAuthority)).count);
}

export async function getVoucherPDA(connection: Connection, tree: PublicKey, leafIndex: number): Promise<PublicKey> {
    let [voucher] = await PublicKey.findProgramAddress(
        [
            Buffer.from("voucher", "utf8"),
            tree.toBuffer(),
            new BN(leafIndex).toBuffer("le", 8),
        ],
        PROGRAM_ID
    );
    return voucher;
}

export async function getCreateTreeIxs(
    connection: Connection,
    maxDepth: number,
    maxBufferSize: number,
    canopyDepth: number,
    payer: PublicKey,
    merkleRoll: PublicKey,
    treeCreator: PublicKey,
): Promise<TransactionInstruction[]> {
    const allocAccountIx = await createAllocTreeIx(
        connection,
        maxBufferSize,
        maxDepth,
        canopyDepth,
        payer,
        merkleRoll,
    );
    const authority = await getBubblegumAuthorityPDA(merkleRoll);
    const initGummyrollIx = createCreateTreeInstruction(
        {
            treeCreator,
            payer,
            authority,
            candyWrapper: CANDY_WRAPPER_PROGRAM_ID,
            gummyrollProgram: GUMMYROLL_PROGRAM_ID,
            merkleSlab: merkleRoll,
        },
        {
            maxDepth,
            maxBufferSize,
        }
    );
    return [allocAccountIx, initGummyrollIx];
}
