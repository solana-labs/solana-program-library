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
    // TODO:- updateAuthority is an optional public key, so typically in Rust we'd represent it with Option<Pubkey>, but we're using OptionalNonZeroPubkey, which just means for None it's going to be a 32-length array of 0.
    updateAuthority: PublicKey;
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

