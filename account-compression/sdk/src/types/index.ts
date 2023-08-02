import { PublicKey } from '@solana/web3.js';
import BN from 'bn.js';

import { PathNode } from '../generated';
export * from './Path';
export * from './Canopy';
export * from './ConcurrentMerkleTree';

export type ChangeLogEventV1 = {
    treeId: PublicKey;
    path: PathNode[];
    seq: BN;
    index: number;
};
