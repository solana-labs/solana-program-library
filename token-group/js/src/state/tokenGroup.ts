import { PublicKey } from '@solana/web3.js';
import { getBytesCodec, getStructCodec } from '@solana/codecs-data-structures';

const tokenGroupCodec = getStructCodec([
    ['updateAuthority', getBytesCodec({ size: 32 })],
    ['mint', getBytesCodec({ size: 32 })],
    ['size', getBytesCodec({ size: 4 })],
    ['maxSize', getBytesCodec({ size: 4 })],
]);

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

export function numberToU32Buffer(num: number): Buffer {
    const buffer = Buffer.alloc(4);
    buffer.writeUInt32LE(num);
    return buffer;
}

// Pack TokenGroup into byte slab
export const packTokenGroup = (group: TokenGroup): Uint8Array => {
    // If no updateAuthority given, set it to the None/Zero PublicKey for encoding
    const updateAuthority = group.updateAuthority ?? PublicKey.default;
    return tokenGroupCodec.encode({
        updateAuthority: updateAuthority.toBuffer(),
        mint: group.mint.toBuffer(),
        size: numberToU32Buffer(group.size),
        maxSize: numberToU32Buffer(group.maxSize),
    });
};

// unpack byte slab into TokenGroup
export function unpackTokenGroup(buffer: Buffer | Uint8Array): TokenGroup {
    const data = tokenGroupCodec.decode(buffer);

    return isNonePubkey(data.updateAuthority)
        ? {
              mint: new PublicKey(data.mint),
              size: Buffer.from(data.size).readUInt32LE(),
              maxSize: Buffer.from(data.maxSize).readUInt32LE(),
          }
        : {
              updateAuthority: new PublicKey(data.updateAuthority),
              mint: new PublicKey(data.mint),
              size: Buffer.from(data.size).readUInt32LE(),
              maxSize: Buffer.from(data.maxSize).readUInt32LE(),
          };
}
