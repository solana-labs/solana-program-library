import { AccountInfo, PublicKey } from '@solana/web3.js';
import BN from 'bn.js';
import { blob, Layout } from 'buffer-layout';

export type Parser<T> = (
    pubkey: PublicKey,
    info: AccountInfo<Buffer>
) =>
    | {
          pubkey: PublicKey;
          account: AccountInfo<Buffer>;
          info: T;
      }
    | undefined;

/** @internal */
export const publicKey = (property = 'publicKey'): Layout<PublicKey> => {
    const layout = blob(32, property);

    const _decode = layout.decode.bind(layout);
    const _encode = layout.encode.bind(layout);

    const publicKeyLayout = layout as Layout<unknown> as Layout<PublicKey>;

    publicKeyLayout.decode = (buffer: Buffer, offset: number) => {
        const data = _decode(buffer, offset);
        return new PublicKey(data);
    };

    publicKeyLayout.encode = (key: PublicKey, buffer: Buffer, offset: number) => {
        return _encode(key.toBuffer(), buffer, offset);
    };

    return publicKeyLayout;
};

/** @internal */
export const bn =
    (length: number) =>
    (property = 'bn'): Layout<BN> => {
        const layout = blob(length, property);

        const _decode = layout.decode.bind(layout);
        const _encode = layout.encode.bind(layout);

        const bnLayout = layout as Layout<unknown> as Layout<BN>;

        bnLayout.decode = (buffer: Buffer, offset: number) => {
            const src = _decode(buffer, offset);
            return new BN(
                [...src]
                    .reverse()
                    .map((i) => `00${i.toString(16)}`.slice(-2))
                    .join(''),
                16
            );
        };

        bnLayout.encode = (bn: BN, buffer: Buffer, offset: number) => {
            const reverse = bn.toArray().reverse();
            let src = Buffer.from(reverse);
            if (src.length !== length) {
                const zeroPad = Buffer.alloc(length);
                src.copy(zeroPad);
                src = zeroPad;
            }
            return _encode(src, buffer, offset);
        };

        return bnLayout;
    };

/** @internal */
export const u64 = bn(8);

/** @internal */
export const u128 = bn(16);
