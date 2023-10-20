import { getArrayDecoder, getBytesDecoder, getStructDecoder, getTupleDecoder } from '@solana/codecs-data-structures';
import { getStringDecoder } from '@solana/codecs-strings';
import { TlvState } from '@solana/spl-type-length-value';
import { PublicKey } from '@solana/web3.js';

import { TokenMetadataError } from './errors.js';

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

export function unpack(buffer: Buffer): TokenMetadata {
    const tlv = new TlvState(buffer, 8, 4);
    const bytes = tlv.firstBytes(TOKEN_METADATA_DISCRIMINATOR);
    if (bytes === null) {
        throw new TokenMetadataError('Invalid Data');
    }
    const decoder = getStructDecoder([
        ['updateAuthority', getBytesDecoder({ size: 32 })],
        ['mint', getBytesDecoder({ size: 32 })],
        ['name', getStringDecoder()],
        ['symbol', getStringDecoder()],
        ['uri', getStringDecoder()],
        ['additionalMetadata', getArrayDecoder(getTupleDecoder([getStringDecoder(), getStringDecoder()]))],
    ]);

    const data = decoder.decode(bytes);

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
