import * as beet from '@metaplex-foundation/beet';
import * as beetSolana from '@metaplex-foundation/beet-solana';
import { PublicKey } from '@solana/web3.js';

/**
 * Canopy fields necessary for deserializing an on-chain Path
 * used in an {@link ConcurrentMerkleTree}
 */
export type Path = {
    // u32
    _padding: number;
    index: number;
    leaf: PublicKey;
    proof: PublicKey[]; // u32
};

/**
 * Factory function for generating a `beet` that can deserialize
 * an on-chain {@link Path}
 *
 * @param maxDepth
 * @returns
 */
export const pathBeetFactory = (maxDepth: number) => {
    return new beet.BeetArgsStruct<Path>(
        [
            ['proof', beet.uniformFixedSizeArray(beetSolana.publicKey, maxDepth)],
            ['leaf', beetSolana.publicKey],
            ['index', beet.u32],
            ['_padding', beet.u32],
        ],
        'Path',
    );
};
