import type { PublicKey } from '@solana/web3.js';

/** The field can be one of the required fields (name, symbol, URI), or a
 * totally new field denoted by a "key" string. */
type Field = 'name' | 'symbol' | 'uri' | string;

export interface TokenMetadata {
    // The authority that can sign to update the metadata
    updateAuthority: PublicKey;
    // The associated mint, used to counter spoofing to be sure that metadata belongs to a particular mint
    mint: PublicKey;
    // The longer name of the token
    name: string;
    // The shortened symbol for the token
    symbol: string;
    /// The URI pointing to richer metadata
    uri: string;
    /// Any additional metadata about the token as key-value pairs
    additionalMetadata: [string, string][];
}

export interface TokenMetadataInstruction {
    /** If the field does not exist on the account, it will be created.
     * If the field does exist, it will be overwritten. */
    updateField: (
        programId: PublicKey,
        metadata: PublicKey,
        updateAuthority: PublicKey,
        field: Field,
        value: string
    ) => Promise<void>;

    /** Removes a key-value pair in a token-metadata account. This only applies
     * to additional fields, and not the base name / symbol / URI fields. */
    removeKey: (
        programId: PublicKey,
        metadata: PublicKey,
        updateAuthority: PublicKey,
        field: Field,
        idempotent: boolean
    ) => Promise<void>;

    /** Updates the token-metadata authority */
    updateAuthority: (
        programId: PublicKey,
        metadata: PublicKey,
        oldAuthority: PublicKey,
        newAuthority: PublicKey
    ) => Promise<void>;

    // Emits the token-metadata as return data
    emit: (programId: PublicKey, metadata: PublicKey) => Promise<TokenMetadata>;
}
