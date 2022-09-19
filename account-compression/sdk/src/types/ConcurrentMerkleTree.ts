
import * as beet from '@metaplex-foundation/beet';
import {
    ChangeLog,
    changeLogBeetFactory
} from './ChangeLog'
import {
    Path,
    pathBeetFactory
} from './Path';

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