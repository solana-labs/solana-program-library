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
 * @param canopyDepth
 * @returns
 */
export const canopyBeetFactory = (canopyDepth: number) => {
  return new beet.BeetArgsStruct<Canopy>(
    [
      [
        'canopyBytes',
        beet.uniformFixedSizeArray(
          beet.u8,
          Math.max(((1 << (canopyDepth + 1)) - 2) * 32, 0)
        ),
      ],
    ],
    'Canopy'
  );
};
