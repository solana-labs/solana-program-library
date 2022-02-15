import { AccountInfo, PublicKey } from '@solana/web3.js';
import BigNumber from 'bignumber.js';
import { blob, Layout, u8 } from 'buffer-layout';
import { toBigIntLE, toBufferLE } from 'bigint-buffer';
import { WAD } from '../constants';

export type Parser<T> = (
    pubkey: PublicKey,
    info: AccountInfo<Buffer>
) =>
    | {
          pubkey: PublicKey;
          info: AccountInfo<Buffer>;
          data: T;
      }
    | undefined;

/** @internal */
export interface EncodeDecode<T> {
    decode: (buffer: Buffer, offset?: number) => T;
    encode: (src: T, buffer: Buffer, offset?: number) => number;
}

/** @internal */
export const encodeDecode = <T>(layout: Layout<T>): EncodeDecode<T> => {
    const decode = layout.decode.bind(layout);
    const encode = layout.encode.bind(layout);
    return { decode, encode };
};

/** @internal */
export const bool = (property = 'bool'): Layout<boolean> => {
    const layout = u8(property);
    const { encode, decode } = encodeDecode(layout);

    const boolLayout = layout as Layout<unknown> as Layout<boolean>;

    boolLayout.decode = (buffer: Buffer, offset: number) => {
        const src = decode(buffer, offset);
        return !!src;
    };

    boolLayout.encode = (bool: boolean, buffer: Buffer, offset: number) => {
        const src = Number(bool);
        return encode(src, buffer, offset);
    };

    return boolLayout;
};

/** @internal */
export const publicKey = (property = 'publicKey'): Layout<PublicKey> => {
    const layout = blob(32, property);
    const { encode, decode } = encodeDecode(layout);

    const publicKeyLayout = layout as Layout<unknown> as Layout<PublicKey>;

    publicKeyLayout.decode = (buffer: Buffer, offset: number) => {
        const src = decode(buffer, offset);
        return new PublicKey(src);
    };

    publicKeyLayout.encode = (publicKey: PublicKey, buffer: Buffer, offset: number) => {
        const src = publicKey.toBuffer();
        return encode(src, buffer, offset);
    };

    return publicKeyLayout;
};

/** @internal */
export const bigInt =
    (length: number) =>
    (property = 'bigInt'): Layout<bigint> => {
        const layout = blob(length, property);
        const { encode, decode } = encodeDecode(layout);

        const bigIntLayout = layout as Layout<unknown> as Layout<bigint>;

        bigIntLayout.decode = (buffer: Buffer, offset: number) => {
            const src = decode(buffer, offset);
            return toBigIntLE(src as Buffer);
        };

        bigIntLayout.encode = (bigInt: bigint, buffer: Buffer, offset: number) => {
            const src = toBufferLE(bigInt, length);
            return encode(src, buffer, offset);
        };

        return bigIntLayout;
    };

/** @internal */
export const u64 = bigInt(8);

/** @internal */
export const u128 = bigInt(16);

/** @internal */
export const decimal = (property = 'decimal'): Layout<BigNumber> => {
    const layout = u128(property);
    const { encode, decode } = encodeDecode(layout);

    const decimalLayout = layout as Layout<unknown> as Layout<BigNumber>;

    decimalLayout.decode = (buffer: Buffer, offset: number) => {
        const src = decode(buffer, offset).toString();
        return new BigNumber(src).div(WAD);
    };

    decimalLayout.encode = (decimal: BigNumber, buffer: Buffer, offset: number) => {
        const src = BigInt(decimal.times(WAD).integerValue().toString());
        return encode(src, buffer, offset);
    };

    return decimalLayout;
};
