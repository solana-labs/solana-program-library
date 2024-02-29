import { PublicKey } from '@solana/web3.js';
import { getBytesCodec, getStructCodec, getU32Codec } from '@solana/codecs';

const tokenGroupCodec = getStructCodec([
    ['updateAuthority', getBytesCodec({ size: 32 })],
    ['mint', getBytesCodec({ size: 32 })],
    ['size', getU32Codec()],
    ['maxSize', getU32Codec()],
]);

export const TOKEN_GROUP_SIZE = tokenGroupCodec.fixedSize;

export interface TokenGroup {
    /** The authority that can sign to update the group */
    updateAuthority?: PublicKey;
    /** The associated mint, used to counter spoofing to be sure that group belongs to a particular mint */
    mint: PublicKey;
    /** The current number of group members */
    size: number;
    /** The maximum number of group members */
    maxSize: number;
}

// Checks if all elements in the array are 0
function isNonePubkey(buffer: Uint8Array): boolean {
    for (let i = 0; i < buffer.length; i++) {
        if (buffer[i] !== 0) {
            return false;
        }
    }
    return true;
}

// Pack TokenGroup into byte slab
export function packTokenGroup(group: TokenGroup): Uint8Array {
    // If no updateAuthority given, set it to the None/Zero PublicKey for encoding
    const updateAuthority = group.updateAuthority ?? PublicKey.default;
    return tokenGroupCodec.encode({
        updateAuthority: updateAuthority.toBuffer(),
        mint: group.mint.toBuffer(),
        size: group.size,
        maxSize: group.maxSize,
    });
}

// unpack byte slab into TokenGroup
export function unpackTokenGroup(buffer: Buffer | Uint8Array): TokenGroup {
    const data = tokenGroupCodec.decode(buffer);

    return isNonePubkey(data.updateAuthority)
        ? {
              mint: new PublicKey(data.mint),
              size: data.size,
              maxSize: data.maxSize,
          }
        : {
              updateAuthority: new PublicKey(data.updateAuthority),
              mint: new PublicKey(data.mint),
              size: data.size,
              maxSize: data.maxSize,
          };
}
