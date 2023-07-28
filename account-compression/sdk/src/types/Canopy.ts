import * as beet from '@metaplex-foundation/beet';

/**
 * Canopy fields necessary for deserializing an on-chain Canopy
 * for a {@link ConcurrentMerkleTreeAccount}
 */
export type Canopy = {
    canopyBytes: number[];
};

/**
 * Factory function for generating a `beet` that can deserialize
 * an on-chain {@link Canopy}
 *
 * {@link Canopy} of depth `N` is an on-chain cache of the top
 * `N` nodes in the {@link ConcurrentMerkleTree}. This is a total
 * of `2^(N+1) - 1` nodes. Each node has `32` bytes.
 * However, the current root of the tree is always stored in the
 * most recent {@link ChangeLog}, so we only need to cache the remaining `N-1` levels.
 *
 * The final formula for account size in bytes: `(2^(N) - 1 - 1) * 32`.
 *
 * @param canopyDepth
 * @returns
 */
export const canopyBeetFactory = (canopyDepth: number) => {
    return new beet.BeetArgsStruct<Canopy>(
        [['canopyBytes', beet.uniformFixedSizeArray(beet.u8, Math.max(((1 << (canopyDepth + 1)) - 2) * 32, 0))]],
        'Canopy'
    );
};
