import { PublicKey } from '@solana/web3.js';
import * as beetSolana from '@metaplex-foundation/beet-solana';
import * as beet from '@metaplex-foundation/beet';

export type ChangeLog = {
    root: PublicKey,
    pathNodes: PublicKey[];
    index: number; // u32
    _padding: number; // u32
};

export const changeLogBeetFactory = (maxDepth: number) => {
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