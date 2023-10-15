import { PublicKey } from '@solana/web3.js';

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
