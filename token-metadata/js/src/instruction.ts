import type { PublicKey } from '@solana/web3.js';

import { Field, TokenMetadata } from './state';


/*
Not implimented as a strict interface, by function signatures so that the 
library can work with programs that implement the Rust interface.
*/


// TODO:- Library should impliment Pack/unpack helpers
export type Pack = (meta: TokenMetadata) => Promise<Buffer>
export type Unpack = (input: Buffer) => Promise<TokenMetadata>

export type Initialize = (
    programId: PublicKey,
    metadata: PublicKey,
    updateAuthority: PublicKey,
    mint: PublicKey,
    mintAuthority: PublicKey,
    name: string,
    symbol: string,
    uri: string,
) => Promise<void>;

/** If the field does not exist on the account, it will be created.
 * If the field does exist, it will be overwritten. */
export type UpdateField = (
    programId: PublicKey,
    metadata: PublicKey,
    updateAuthority: PublicKey,
    field: Field,
    value: string
) => Promise<void>;

/** Removes a key-value pair in a token-metadata account. This only applies
 * to additional fields, and not the base name / symbol / URI fields. */
export type RemoveKey = (
    programId: PublicKey,
    metadata: PublicKey,
    updateAuthority: PublicKey,
    field: Field,
    idempotent: boolean
) => Promise<void>;

/** Updates the token-metadata authority */
export type UpdateAuthority = (
    programId: PublicKey,
    metadata: PublicKey,
    oldAuthority: PublicKey,
    newAuthority: PublicKey
) => Promise<void>;

// Emits the token-metadata as return data
export type Emit = (programId: PublicKey, metadata: PublicKey) => Promise<TokenMetadata>;
