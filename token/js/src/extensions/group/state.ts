import { struct, u32 } from '@solana/buffer-layout';
import { publicKey } from '@solana/buffer-layout-utils';
import { PublicKey } from '@solana/web3.js';
import type { TokenGroup } from '@solana/spl-token-group';
import type { Mint } from '../../state/mint.js';
import { ExtensionType, getExtensionData } from '../extensionType.js';

/** Buffer layout for de/serializing a TokenGroup extension */
export const TokenGroupLayout = struct<TokenGroup>([
    publicKey('updateAuthority'),
    publicKey('mint'),
    u32('size'),
    u32('maxSize'),
]);

export const TOKEN_GROUP_SIZE = TokenGroupLayout.span;

export function getTokenGroupState(mint: Mint): Partial<TokenGroup> | null {
    const extensionData = getExtensionData(ExtensionType.TokenGroup, mint.tlvData);
    if (extensionData !== null) {
        const { updateAuthority, mint, size, maxSize } = TokenGroupLayout.decode(extensionData);

        // Explicity set None/Zero keys to null
        return {
            updateAuthority: updateAuthority?.equals(PublicKey.default) ? undefined : updateAuthority,
            mint,
            size,
            maxSize,
        };
    } else {
        return null;
    }
}
