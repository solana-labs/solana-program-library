
import * as beet from '@metaplex-foundation/beet';
import { PublicKey } from '@solana/web3.js';
import * as beetSolana from '@metaplex-foundation/beet-solana';
import {
    Path,
    pathBeetFactory
} from './Path';

type ChangeLog = {
    root: PublicKey,
    pathNodes: PublicKey[];
    index: number; // u32
    _padding: number; // u32
};

const changeLogBeetFactory = (maxDepth: number) => {
    return new beet.BeetArgsStruct<ChangeLog>(
        [
            ['root', beetSolana.publicKey],
            ['pathNodes', beet.uniformFixedSizeArray(beetSolana.publicKey, maxDepth)],
            ['index', beet.u32],
            ["_padding", beet.u32],
        ],
        'ChangeLog'
    )
}

export type ConcurrentMerkleTree = {
    sequenceNumber: beet.bignum; // u64
    activeIndex: beet.bignum; // u64
    bufferSize: beet.bignum; // u64
    changeLogs: ChangeLog[];
    rightMostPath: Path;
};

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
}