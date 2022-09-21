import * as beet from '@metaplex-foundation/beet';

export type Canopy = {
    canopyBytes: number[];
}

export const canopyBeetFactory = (canopyDepth: number) => {
    return new beet.BeetArgsStruct<Canopy>(
        [
            ['canopyBytes', beet.uniformFixedSizeArray(beet.u8, Math.max(((1 << canopyDepth + 1) - 2) * 32, 0))],
        ],
        'Canopy'
    );
}