import { BN } from "@project-serum/anchor";
import { TransactionInstruction, PublicKey, Connection, AccountInfo } from "@solana/web3.js";
import { keccak_256 } from "js-sha3";
import { Creator, Nonce, PROGRAM_ID } from './generated';
import { CANDY_WRAPPER_PROGRAM_ID, bufferToArray, num16ToBuffer } from "@sorend-solana/utils";
import { PROGRAM_ID as GUMMYROLL_PROGRAM_ID, createAllocTreeIx } from "@sorend-solana/gummyroll";
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

export async function getLeafAssetId(tree: PublicKey, leafIndex: BN): Promise<PublicKey> {
    let [assetId] = await PublicKey.findProgramAddress(
        [
            Buffer.from("asset", "utf8"),
            tree.toBuffer(),
            leafIndex.toBuffer("le", 8),
        ],
        PROGRAM_ID
    );
    return assetId
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

export function computeMetadataArgsHash(mintIx: TransactionInstruction) {
    const metadataArgsBuffer = mintIx.data.slice(8)
    return keccak_256.digest(metadataArgsBuffer);
}

export function computeDataHash(sellerFeeBasisPoints: number, mintIx?: TransactionInstruction, metadataArgsHash?: number[]) {
    // Input validation
    if (typeof mintIx === 'undefined' && typeof metadataArgsHash === 'undefined') {
        throw new Error("Either the mint NFT instruction or the hash of metadata args must be provided to determine the data hash of the leaf!");
    }
    if (typeof mintIx !== 'undefined' && typeof metadataArgsHash !== 'undefined') {
        throw new Error("Only the mint instruction or the hash of metadata args should be specified, not both");
    }

    if (typeof mintIx !== 'undefined') {
        metadataArgsHash = computeMetadataArgsHash(mintIx);
    }

    if (typeof metadataArgsHash === 'undefined') {
        throw new Error("Metadata Args Hash Unexpectedly Undefined!");
    }

    const sellerFeeBasisPointsNumberArray = bufferToArray(num16ToBuffer(sellerFeeBasisPoints))
    const allDataToHash = metadataArgsHash.concat(sellerFeeBasisPointsNumberArray)
    const dataHashOfCompressedNFT = bufferToArray(
        Buffer.from(keccak_256.digest(allDataToHash))
    );
    return dataHashOfCompressedNFT;
}

export function computeCreatorHash(creators: Creator[]) {
    let bufferOfCreatorData = Buffer.from([]);
    let bufferOfCreatorShares = Buffer.from([]);
    for (let creator of creators) {
        bufferOfCreatorData = Buffer.concat([bufferOfCreatorData, creator.address.toBuffer(), Buffer.from([creator.share])])
        bufferOfCreatorShares = Buffer.concat([bufferOfCreatorShares, Buffer.from([creator.share])])
    }
    let creatorHash = bufferToArray(Buffer.from(keccak_256.digest(bufferOfCreatorData)));
    return creatorHash
}
