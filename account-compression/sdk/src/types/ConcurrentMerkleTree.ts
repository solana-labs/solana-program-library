import * as beet from '@metaplex-foundation/beet';
import * as beetSolana from '@metaplex-foundation/beet-solana';
import { PublicKey } from '@solana/web3.js';

import { Path, pathBeetFactory } from './Path';

/**
 * ChangeLog information necessary for deserializing an on-chain ConcurrentMerkleTree
 * @private
 */
export type ChangeLogInternal = {
    root: PublicKey;
    pathNodes: PublicKey[];
    index: number; // u32
    _padding: number; // u32
};

const changeLogBeetFactory = (maxDepth: number) => {
    return new beet.BeetArgsStruct<ChangeLogInternal>(
        [
            ['root', beetSolana.publicKey],
            ['pathNodes', beet.uniformFixedSizeArray(beetSolana.publicKey, maxDepth)],
            ['index', beet.u32],
            ['_padding', beet.u32],
        ],
        'ChangeLog'
    );
};

/**
 * ConcurrentMerkleTree fields necessary for deserializing an on-chain ConcurrentMerkleTree
 */
export type ConcurrentMerkleTree = {
    sequenceNumber: beet.bignum; // u64
    activeIndex: beet.bignum; // u64
    bufferSize: beet.bignum; // u64
    changeLogs: ChangeLogInternal[];
    rightMostPath: Path;
};

/**
 * Factory function for generating a `beet` that can deserialize
 * an on-chain {@link ConcurrentMerkleTree}
 *
 * @param maxDepth
 * @param maxBufferSize
 * @returns
 */
export const concurrentMerkleTreeBeetFactory = (maxDepth: number, maxBufferSize: number) => {
    return new beet.BeetArgsStruct<ConcurrentMerkleTree>(
        [
            ['sequenceNumber', beet.u64],
            ['activeIndex', beet.u64],
            ['bufferSize', beet.u64],
            ['changeLogs', beet.uniformFixedSizeArray(changeLogBeetFactory(maxDepth), maxBufferSize)],
            ['rightMostPath', pathBeetFactory(maxDepth)],
        ],
        'ConcurrentMerkleTree'
    );
};
