import { PublicKey } from '@solana/web3.js';
import * as beetSolana from '@metaplex-foundation/beet-solana';
import * as beet from '@metaplex-foundation/beet';

export type Path = {
    proof: PublicKey[];
    leaf: PublicKey;
    index: number; // u32
    _padding: number; // u32
};

export const pathBeetFactory = (maxDepth: number) => {
    return new beet.BeetArgsStruct<Path>(
        [
            ['proof', beet.uniformFixedSizeArray(beetSolana.publicKey, maxDepth)],
            ['leaf', beetSolana.publicKey],
            ['index', beet.u32],
            ["_padding", beet.u32],
        ],
        'Path'
    )
}
