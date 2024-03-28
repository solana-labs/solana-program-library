import { PublicKey } from '@solana/web3.js';

export const SPL_NOOP_ADDRESS = 'noopb9bkMVfRPU8AsbpTUg8AQkHtKwMYZiFUjNRtMmV';
export const SPL_NOOP_PROGRAM_ID = new PublicKey(SPL_NOOP_ADDRESS);

/**
 * DepthSizePair is a valid (`maxDepth`, `maxBufferSize`) tuple for an SPL ConcurrentMerkleTree
 * Only the tuples listed in {@link ALL_DEPTH_SIZE_PAIRS} are valid for
 * creating a new {@link ConcurrentMerkleTreeAccount}.
 */
export type DepthSizePair = {
    maxBufferSize: number;
    maxDepth: number;
};

const allPairs: number[][] = [
    [3, 8],
    [5, 8],
    [6, 16],
    [7, 16],
    [8, 16],
    [9, 16],
    [10, 32],
    [11, 32],
    [12, 32],
    [13, 32],
    [14, 64],
    [14, 256],
    [14, 1024],
    [14, 2048],
    [15, 64],
    [16, 64],
    [17, 64],
    [18, 64],
    [19, 64],
    [20, 64],
    [20, 256],
    [20, 1024],
    [20, 2048],
    [24, 64],
    [24, 256],
    [24, 512],
    [24, 1024],
    [24, 2048],
    [26, 512],
    [26, 1024],
    [26, 2048],
    [30, 512],
    [30, 1024],
    [30, 2048],
];

/**
 * Valid pairs for creating a new {@link ConcurrentMerkleTreeAccount}
 */
export const ALL_DEPTH_SIZE_PAIRS: ValidDepthSizePair[] = allPairs.map(pair => {
    return {
        maxBufferSize: pair[1],
        maxDepth: pair[0],
    } as ValidDepthSizePair;
});

export type ValidDepthSizePair =
    | { maxBufferSize: 8; maxDepth: 3 }
    | { maxBufferSize: 8; maxDepth: 5 }
    | { maxBufferSize: 16; maxDepth: 6 }
    | { maxBufferSize: 16; maxDepth: 7 }
    | { maxBufferSize: 16; maxDepth: 8 }
    | { maxBufferSize: 16; maxDepth: 9 }
    | { maxBufferSize: 32; maxDepth: 10 }
    | { maxBufferSize: 32; maxDepth: 11 }
    | { maxBufferSize: 32; maxDepth: 12 }
    | { maxBufferSize: 32; maxDepth: 13 }
    | { maxBufferSize: 64; maxDepth: 14 }
    | { maxBufferSize: 64; maxDepth: 15 }
    | { maxBufferSize: 64; maxDepth: 16 }
    | { maxBufferSize: 64; maxDepth: 17 }
    | { maxBufferSize: 64; maxDepth: 18 }
    | { maxBufferSize: 64; maxDepth: 19 }
    | { maxBufferSize: 64; maxDepth: 20 }
    | { maxBufferSize: 64; maxDepth: 24 }
    | { maxBufferSize: 256; maxDepth: 14 }
    | { maxBufferSize: 256; maxDepth: 20 }
    | { maxBufferSize: 256; maxDepth: 24 }
    | { maxBufferSize: 512; maxDepth: 24 }
    | { maxBufferSize: 512; maxDepth: 26 }
    | { maxBufferSize: 512; maxDepth: 30 }
    | { maxBufferSize: 1024; maxDepth: 14 }
    | { maxBufferSize: 1024; maxDepth: 20 }
    | { maxBufferSize: 1024; maxDepth: 24 }
    | { maxBufferSize: 1024; maxDepth: 26 }
    | { maxBufferSize: 1024; maxDepth: 30 }
    | { maxBufferSize: 2048; maxDepth: 14 }
    | { maxBufferSize: 2048; maxDepth: 20 }
    | { maxBufferSize: 2048; maxDepth: 24 }
    | { maxBufferSize: 2048; maxDepth: 26 }
    | { maxBufferSize: 2048; maxDepth: 30 };
