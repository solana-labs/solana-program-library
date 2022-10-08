import type { PublicKey, Connection, Commitment, GetAccountInfoConfig } from "@solana/web3.js";
import * as borsh from "borsh";
import * as BN from 'bn.js';
import * as beet from '@metaplex-foundation/beet';
import * as beetSolana from '@metaplex-foundation/beet-solana';

import {
    concurrentMerkleTreeHeaderBeet,
    ConcurrentMerkleTreeHeader
} from '../generated/types/ConcurrentMerkleTreeHeader';
import {
    Canopy,
    canopyBeetFactory,
    ConcurrentMerkleTree,
    concurrentMerkleTreeBeetFactory,
} from '../types';
import { ConcurrentMerkleTreeHeaderDataV1, concurrentMerkleTreeHeaderDataV1Beet } from "../generated";

/**
 * These are all the fields needed to deserialize the solana account
 * that the ConcurrentMerkleTree is stored in
 */
export class ConcurrentMerkleTreeAccount {
    public header: ConcurrentMerkleTreeHeader;
    public tree: ConcurrentMerkleTree;
    public canopy: Canopy;

    constructor(header: ConcurrentMerkleTreeHeader, tree: ConcurrentMerkleTree, canopy: Canopy) {
        this.header = header;
        this.tree = tree;
        this.canopy = canopy;
    }

    static fromBuffer(buffer: Buffer): ConcurrentMerkleTreeAccount {
        return deserializeConcurrentMerkleTree(buffer);
    }

    static async fromAccountAddress(connection: Connection, publicKey: PublicKey, commitmentOrConfig?: Commitment | GetAccountInfoConfig): Promise<ConcurrentMerkleTreeAccount> {
        const account = await connection.getAccountInfo(publicKey, commitmentOrConfig);
        if (!account) {
            throw new Error("CMT account data unexpectedly null!");
        }
        return deserializeConcurrentMerkleTree(account.data);
    }

    private getHeaderV1(): ConcurrentMerkleTreeHeaderDataV1 {
        return this.header.header.fields[0];
    }

    getMaxBufferSize(): number {
        return this.getHeaderV1().maxBufferSize;
    }

    getMaxDepth(): number {
        return this.getHeaderV1().maxDepth;
    }

    getBufferSize(): number {
        return new BN.BN(this.tree.bufferSize).toNumber();
    }

    getCurrentRoot(): Buffer {
        return this.tree.changeLogs[this.getActiveIndex()].root.toBuffer();
    }

    getActiveIndex(): number {
        return new BN.BN(this.tree.activeIndex).toNumber();
    }

    getAuthority(): PublicKey {
        return this.getHeaderV1().authority;
    }

    getCreationSlot(): number {
        return new BN.BN(this.getHeaderV1().creationSlot).toNumber();
    }

    getCurrentSeq(): number {
        return new BN.BN(this.tree.sequenceNumber).toNumber();
    }

    getCanopyDepth(): number {
        return getCanopyDepth(this.canopy.canopyBytes.length);
    }

};

export function getCanopyDepth(canopyByteLength: number): number {
    if (canopyByteLength === 0) {
        return 0;
    }
    return Math.log2(canopyByteLength / 32 + 2) - 1
}

function deserializeConcurrentMerkleTree(buffer: Buffer): ConcurrentMerkleTreeAccount {
    let offset = 0;
    const [versionedHeader, offsetIncr] = concurrentMerkleTreeHeaderBeet.deserialize(buffer);
    offset = offsetIncr;

    // Only 1 version available
    if (versionedHeader.header.__kind !== "V1") {
        throw Error(`Header has unsupported version: ${versionedHeader.header.__kind}`);
    }
    const header = versionedHeader.header.fields[0];
    const [tree, offsetIncr2] = concurrentMerkleTreeBeetFactory(header.maxDepth, header.maxBufferSize).deserialize(buffer, offset);
    offset = offsetIncr2;

    const canopyDepth = getCanopyDepth(buffer.byteLength - offset);
    let canopy: Canopy = {
        canopyBytes: []
    }
    if (canopyDepth !== 0) {
        const [deserializedCanopy, offsetIncr3] = canopyBeetFactory(canopyDepth).deserialize(buffer, offset);
        canopy = deserializedCanopy;
        offset = offsetIncr3;
    }

    if (buffer.byteLength !== offset) {
        throw new Error(
            "Failed to process whole buffer when deserializing Merkle Account Data"
        );
    }
    return new ConcurrentMerkleTreeAccount(versionedHeader, tree, canopy);
}

export function getConcurrentMerkleTreeAccountSize(
    maxDepth: number,
    maxBufferSize: number,
    canopyDepth?: number,
    headerVersion: string = "V1",
): number {
    if (headerVersion != "V1") {
        throw Error("Unsupported header version")
    }

    return 2 + concurrentMerkleTreeHeaderDataV1Beet.byteSize +
        concurrentMerkleTreeBeetFactory(maxDepth, maxBufferSize).byteSize +
        (canopyDepth ? canopyBeetFactory(canopyDepth).byteSize : 0);
}