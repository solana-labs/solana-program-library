import type { Commitment, Connection, GetAccountInfoConfig, PublicKey } from '@solana/web3.js';
import { BN } from 'bn.js';

import { ConcurrentMerkleTreeHeaderDataV1, concurrentMerkleTreeHeaderDataV1Beet } from '../generated';
import {
    ConcurrentMerkleTreeHeader,
    concurrentMerkleTreeHeaderBeet,
} from '../generated/types/ConcurrentMerkleTreeHeader';
import { Canopy, canopyBeetFactory, ConcurrentMerkleTree, concurrentMerkleTreeBeetFactory } from '../types';

/**
 * This class provides all the getter methods to deserialize
 * information associated with an on-chain ConcurrentMerkleTree
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

    static async fromAccountAddress(
        connection: Connection,
        publicKey: PublicKey,
        commitmentOrConfig?: Commitment | GetAccountInfoConfig
    ): Promise<ConcurrentMerkleTreeAccount> {
        const account = await connection.getAccountInfo(publicKey, commitmentOrConfig);
        if (!account) {
            throw new Error('CMT account data unexpectedly null!');
        }
        return deserializeConcurrentMerkleTree(account.data);
    }

    private getHeaderV1(): ConcurrentMerkleTreeHeaderDataV1 {
        return this.header.header.fields[0];
    }

    /**
     * Returns the `maxBufferSize` for this tree, by reading the account's header
     * @returns
     */
    getMaxBufferSize(): number {
        return this.getHeaderV1().maxBufferSize;
    }

    /**
     * Returns the `maxDepth` of this tree, by reading the account's header
     * @returns
     */
    getMaxDepth(): number {
        return this.getHeaderV1().maxDepth;
    }

    /**
     * Returns `min(seq, maxBufferSize)`
     * @returns
     */
    getBufferSize(): number {
        return new BN.BN(this.tree.bufferSize).toNumber();
    }

    /**
     * Returns the current root hash for this on-chain tree
     * @returns
     */
    getCurrentRoot(): Buffer {
        return this.tree.changeLogs[this.getCurrentBufferIndex()].root.toBuffer();
    }

    /**
     * Returns the index to the spot in the on-chain buffer that stores the current
     * root and last changelog.
     *
     * Should always be `this.getCurrentSeq() % this.getMaxBufferSize()`
     * @returns
     */
    getCurrentBufferIndex(): number {
        return new BN.BN(this.tree.activeIndex).toNumber();
    }

    /**
     * Returns the PublicKey that can execute modifying operations
     * on this tree
     * @returns
     */
    getAuthority(): PublicKey {
        return this.getHeaderV1().authority;
    }

    /**
     * Returns the slot that this tree was created in. Useful for indexing
     * transactions associated with this tree.
     * @returns
     */
    getCreationSlot() {
        return new BN(this.getHeaderV1().creationSlot);
    }

    /**
     * Returns the number of modifying operations that have been performed
     * on this tree.
     * @returns
     */
    getCurrentSeq() {
        return new BN(this.tree.sequenceNumber);
    }

    /**
     * Returns the depth of the on-chain tree-cache. Increasing the canopy depth reduces the size of the proofs
     * that have to be passed for tree instructions.
     * @returns the size
     */
    getCanopyDepth(): number {
        return getCanopyDepth(this.canopy.canopyBytes.length);
    }
}

/**
 * Return expected depth of the cached {@link Canopy} tree just from the number
 * of bytes used to store the Canopy
 *
 * @param canopyByteLength
 * @returns
 */
export function getCanopyDepth(canopyByteLength: number): number {
    if (canopyByteLength === 0) {
        return 0;
    }
    return Math.log2(canopyByteLength / 32 + 2) - 1;
}

function deserializeConcurrentMerkleTree(buffer: Buffer): ConcurrentMerkleTreeAccount {
    let offset = 0;
    const [versionedHeader, offsetIncr] = concurrentMerkleTreeHeaderBeet.deserialize(buffer);
    offset = offsetIncr;

    // Only 1 version available
    if (versionedHeader.header.__kind !== 'V1') {
        throw Error(`Header has unsupported version: ${versionedHeader.header.__kind}`);
    }
    const header = versionedHeader.header.fields[0];
    const [tree, offsetIncr2] = concurrentMerkleTreeBeetFactory(header.maxDepth, header.maxBufferSize).deserialize(
        buffer,
        offset
    );
    offset = offsetIncr2;

    const canopyDepth = getCanopyDepth(buffer.byteLength - offset);
    let canopy: Canopy = {
        canopyBytes: [],
    };
    if (canopyDepth !== 0) {
        const [deserializedCanopy, offsetIncr3] = canopyBeetFactory(canopyDepth).deserialize(buffer, offset);
        canopy = deserializedCanopy;
        offset = offsetIncr3;
    }

    if (buffer.byteLength !== offset) {
        throw new Error('Failed to process whole buffer when deserializing Merkle Account Data');
    }
    return new ConcurrentMerkleTreeAccount(versionedHeader, tree, canopy);
}

/**
 * Calculate the expected size of an ConcurrentMerkleTreeAccount
 * @param maxDepth
 * @param maxBufferSize
 * @param canopyDepth
 * @param headerVersion
 * @returns
 */
export function getConcurrentMerkleTreeAccountSize(
    maxDepth: number,
    maxBufferSize: number,
    canopyDepth?: number,
    headerVersion = 'V1'
): number {
    if (headerVersion != 'V1') {
        throw Error('Unsupported header version');
    }

    // The additional 2 bytes are needed for
    // - the account disciminant  (1 byte)
    // - the header version       (1 byte)
    return (
        2 +
        concurrentMerkleTreeHeaderDataV1Beet.byteSize +
        concurrentMerkleTreeBeetFactory(maxDepth, maxBufferSize).byteSize +
        (canopyDepth ? canopyBeetFactory(canopyDepth).byteSize : 0)
    );
}
