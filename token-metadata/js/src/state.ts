import { PublicKey } from '@solana/web3.js';
import {
    getArrayDecoder,
    getArrayEncoder,
    getBytesDecoder,
    getBytesEncoder,
    getStructDecoder,
    getStructEncoder,
    getTupleDecoder,
    getTupleEncoder,
} from '@solana/codecs-data-structures';
import { getStringDecoder, getStringEncoder } from '@solana/codecs-strings';

export const TOKEN_METADATA_DISCRIMINATOR = Buffer.from([112, 132, 90, 90, 11, 88, 157, 87]);

export interface TokenMetadata {
    // The authority that can sign to update the metadata
    updateAuthority?: PublicKey;
    // The associated mint, used to counter spoofing to be sure that metadata belongs to a particular mint
    mint: PublicKey;
    // The longer name of the token
    name: string;
    // The shortened symbol for the token
    symbol: string;
    // The URI pointing to richer metadata
    uri: string;
    // Any additional metadata about the token as key-value pairs
    additionalMetadata: [string, string][];
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

// Pack TokenMetadata into byte slab
export const pack = (meta: TokenMetadata): Uint8Array => {
    const encoder = getStructEncoder([
        ['updateAuthority', getBytesEncoder({ size: 32 })],
        ['mint', getBytesEncoder({ size: 32 })],
        ['name', getStringEncoder()],
        ['symbol', getStringEncoder()],
        ['uri', getStringEncoder()],
        ['additionalMetadata', getArrayEncoder(getTupleEncoder([getStringEncoder(), getStringEncoder()]))],
    ]);

    // If no updateAuthority given, set it to the None/Zero PublicKey for encoding
    const updateAuthority = meta.updateAuthority ?? PublicKey.default;
    return encoder.encode({
        ...meta,
        updateAuthority: updateAuthority.toBuffer(),
        mint: meta.mint.toBuffer(),
    });
};

// unpack byte slab into TokenMetadata
export function unpack(buffer: Buffer | Uint8Array): TokenMetadata {
    const decoder = getStructDecoder([
        ['updateAuthority', getBytesDecoder({ size: 32 })],
        ['mint', getBytesDecoder({ size: 32 })],
        ['name', getStringDecoder()],
        ['symbol', getStringDecoder()],
        ['uri', getStringDecoder()],
        ['additionalMetadata', getArrayDecoder(getTupleDecoder([getStringDecoder(), getStringDecoder()]))],
    ]);

    const data = decoder.decode(buffer);

    return isNonePubkey(data[0].updateAuthority)
        ? {
              mint: new PublicKey(data[0].mint),
              name: data[0].name,
              symbol: data[0].symbol,
              uri: data[0].uri,
              additionalMetadata: data[0].additionalMetadata,
          }
        : {
              updateAuthority: new PublicKey(data[0].updateAuthority),
              mint: new PublicKey(data[0].mint),
              name: data[0].name,
              symbol: data[0].symbol,
              uri: data[0].uri,
              additionalMetadata: data[0].additionalMetadata,
          };
}
