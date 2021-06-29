import BN from 'bn.js';
import { struct, u8 } from 'buffer-layout';
import { u64 } from '../util';

export interface LastUpdate {
    slot: BN;
    stale: boolean;
}

export const LastUpdateLayout = struct<LastUpdate>([u64('slot'), u8('stale')], 'lastUpdate');
