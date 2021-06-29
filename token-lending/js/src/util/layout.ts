import { PublicKey } from '@solana/web3.js';
import BN from 'bn.js';
import { blob, Layout, offset, struct, u32 } from 'buffer-layout';

export const publicKey = (property = 'publicKey') => {
    const layout = blob(32, property);

    const _decode = layout.decode.bind(layout);
    const _encode = layout.encode.bind(layout);

    const publicKeyLayout = layout as Layout<any> as Layout<PublicKey>;

    publicKeyLayout.decode = (buffer: Buffer, offset: number) => {
        const data = _decode(buffer, offset);
        return new PublicKey(data);
    };

    publicKeyLayout.encode = (key: PublicKey, buffer: Buffer, offset: number) => {
        return _encode(key.toBuffer(), buffer, offset);
    };

    return publicKeyLayout;
};

export const bn =
    (length: number) =>
    (property = 'bn') => {
        const layout = blob(length, property);

        const _decode = layout.decode.bind(layout);
        const _encode = layout.encode.bind(layout);

        const bnLayout = layout as Layout<any> as Layout<BN>;

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

export const u64 = bn(8);

export const u128 = bn(16);

interface RustString {
    length: number;
    lengthPadding: number;
    chars: Buffer;
}

/**
 * Layout for a Rust String type
 */
export const rustString = (property = 'string') => {
    const layout = struct<RustString>(
        [u32('length'), u32('lengthPadding'), blob(offset(u32(), -8), 'chars')],
        property
    );

    const _decode = layout.decode.bind(layout);
    const _encode = layout.encode.bind(layout);

    const stringLayout = layout as Layout<any> as Layout<string>;

    stringLayout.decode = (buffer: Buffer, offset: number) => {
        const data = _decode(buffer, offset);
        return data.chars.toString('utf8');
    };

    stringLayout.encode = (str: string, buffer: Buffer, offset: number) => {
        // @TODO: does this need length/padding?
        const data = {
            chars: Buffer.from(str, 'utf8'),
        } as RustString;
        return _encode(data, buffer, offset);
    };

    return stringLayout;
};
