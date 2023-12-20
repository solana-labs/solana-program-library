import { PublicKey } from '@solana/web3.js';
import { getBytesCodec, getStructCodec } from '@solana/codecs-data-structures';

const tokenGroupCodec = getStructCodec([
    ['updateAuthority', getBytesCodec({ size: 32 })],
    ['mint', getBytesCodec({ size: 32 })],
    ['size', getBytesCodec({ size: 4 })],
    ['maxSize', getBytesCodec({ size: 4 })],
]);

export class PodU32 {
    #value: number;
    constructor(value: number) {
        this.#value = value;
    }
    toBuffer(): Buffer {
        const buffer = Buffer.alloc(4);
        buffer.writeUInt32LE(this.#value);
        return buffer;
    }
}

export interface TokenGroup {
    /** The authority that can sign to update the group */
    updateAuthority?: PublicKey;
    /** The associated mint, used to counter spoofing to be sure that group belongs to a particular mint */
    mint: PublicKey;
    /** The current number of group members */
    size: PodU32;
    /** The maximum number of group members */
    maxSize: PodU32;
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
export const packTokenGroup = (group: TokenGroup): Uint8Array => {
    // If no updateAuthority given, set it to the None/Zero PublicKey for encoding
    const updateAuthority = group.updateAuthority ?? PublicKey.default;
    return tokenGroupCodec.encode({
        updateAuthority: updateAuthority.toBuffer(),
        mint: group.mint.toBuffer(),
        size: group.size.toBuffer(),
        maxSize: group.maxSize.toBuffer(),
    });
};

// unpack byte slab into TokenGroup
export function unpackTokenGroup(buffer: Buffer | Uint8Array): TokenGroup {
    const data = tokenGroupCodec.decode(buffer);

    return isNonePubkey(data.updateAuthority)
        ? {
              mint: new PublicKey(data.mint),
              size: new PodU32(data.size),
              maxSize: new PodU32(data.maxSize),
          }
        : {
              updateAuthority: new PublicKey(data.updateAuthority),
              mint: new PublicKey(data.mint),
              size: new PodU32(data.size),
              maxSize: new PodU32(data.maxSize),
          };
}
