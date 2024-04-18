import { blob } from '@solana/buffer-layout';
import type { Layout } from '@solana/buffer-layout';
import { encodeDecode } from '@solana/buffer-layout-utils';
import { PodElGamalPubkey } from 'solana-zk-token-sdk-experimental';

export const elgamalPublicKey = (property?: string): Layout<PodElGamalPubkey> => {
    const layout = blob(32, property);
    const { encode, decode } = encodeDecode(layout);

    const elgamalPublicKeyLayout = layout as Layout<unknown> as Layout<PodElGamalPubkey>;

    elgamalPublicKeyLayout.decode = (buffer: Buffer, offset: number) => {
        const src = decode(buffer, offset);
        return new PodElGamalPubkey(src);
    };

    elgamalPublicKeyLayout.encode = (elgamalPublicKey: PodElGamalPubkey, buffer: Buffer, offset: number) => {
        const src = elgamalPublicKey.toBytes();
        return encode(src, buffer, offset);
    };

    return elgamalPublicKeyLayout;
};
