import {
    PublicKey
} from '@solana/web3.js';
import * as borsh from 'borsh';
import { bignum } from '@metaplex-foundation/beet'
import { BN } from '@project-serum/anchor';

export function readPublicKey(reader: borsh.BinaryReader): PublicKey {
    return new PublicKey(reader.readFixedArray(32));
}

export function val(num: bignum): BN {
    if (BN.isBN(num)) {
        return num;
    }
    return new BN(num);
}

export function strToByteArray(str: string): number[] {
    return [...str].reduce((acc, c, ind) => acc.concat([str.charCodeAt(ind)]), []);
}

export function strToByteUint8Array(str: string): Uint8Array {
    return Uint8Array.from([...str].reduce((acc, c, ind) => acc.concat([str.charCodeAt(ind)]), []));
}