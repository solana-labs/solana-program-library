import { PublicKey } from '@solana/web3.js';
import { deserialize } from 'borsh';
import { TlvState } from '@solana/spl-type-length-value';
import { TokenMetadataError } from './errors';

/** The field can be one of the required fields (name, symbol, URI), or a
 * totally new field denoted by a "key" string.
 *
 * Define explicitly o make it abundantly clear that 'name' | 'symbol' | 'uri' are fundamental parts of the interface,
 * while any other key is additional
 */

export type Field = 'name' | 'symbol' | 'uri' | string;

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

export const schema = {
    struct: {
        updateAuthority: { array: { type: 'u8', len: 32 } },
        mint: { array: { type: 'u8', len: 32 } },
        name: 'string',
        symbol: 'string',
        uri: 'string',
        additionalMetadata: { array: { type: { array: { type: 'string', len: 2 } } } },
    },
};

// From https://github.com/solana-labs/solana-program-library/blob/master/token-metadata/interface/src/state.rs#L45C43-L45C43
export const TokenMetadataDiscriminate = Buffer.from([112, 132, 90, 90, 11, 88, 157, 87]);

// buffer is a tlv |--discriminate--|--length--|--bytes--|
// Where bytes is serialised Tokenmetadata with Borsh
export function unpack(buffer: Buffer): TokenMetadata {
    const tlv = new TlvState(buffer, 8, 4);
    const bytes = tlv.firstBytes(TokenMetadataDiscriminate);
    if (bytes === null) {
        throw new TokenMetadataError('Invalid Data');
    }
    const data = deserialize(schema, bytes) as any;

    const meta: TokenMetadata = {
        updateAuthority: new PublicKey(data.updateAuthority as Buffer),
        mint: new PublicKey(data.mint as Buffer),
        name: data.name,
        symbol: data.symbol,
        uri: data.uri,
        additionalMetadata: data.additionalMetadata,
    };

    if (meta.updateAuthority && meta.updateAuthority.toString() === PublicKey.default.toString()) {
        // If update Authority is empty, treat as if it doesn't exist
        delete meta.updateAuthority;
    }

    return meta;
}
