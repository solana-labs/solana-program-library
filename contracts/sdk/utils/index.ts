import {
    PublicKey
} from '@solana/web3.js';
import * as borsh from 'borsh';

export function readPublicKey(reader: borsh.BinaryReader): PublicKey {
    return new PublicKey(reader.readFixedArray(32));
}