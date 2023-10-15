import { PublicKey, TransactionInstruction } from '@solana/web3.js';
import { splDiscriminate } from '@solana/spl-type-length-value';
import { serialize } from 'borsh';

import { Field } from './state';

// Taken from https://github.com/solana-labs/solana-program-library/blob/master/token-metadata/interface/src/instruction.rs
export const DISCRIMNATOR = {
    Initialize: splDiscriminate('spl_token_metadata_interface:initialize_account'),
    UpdateField: splDiscriminate('spl_token_metadata_interface:updating_field'),
    RemoveKey: splDiscriminate('spl_token_metadata_interface:remove_key_ix'),
    UpdateAuthorithy: splDiscriminate('spl_token_metadata_interface:update_the_authority'),
    Emit: splDiscriminate('spl_token_metadata_interface:emitter'),
};

// Order of keys matters
export const SCHEMA = {
    Initialize: { struct: { name: 'string', symbol: 'string', uri: 'string' } },
    UpdateField: { struct: { field: 'string', value: 'string' } },
    RemoveKey: { struct: { idempotent: 'bool', key: 'string' } },
    UpdateAuthorithy: { struct: { newAuthority: { array: { type: 'u8', len: 32 } } } },
    Emit: { struct: { start: { option: 'u64' }, end: { option: 'u64' } } },
};

/* 
    Initializes a TLV entry with the basic token-metadata fields.
    
    Assumes that the provided mint is an SPL token mint, that the metadata
    account is allocated and assigned to the program, and that the metadata
    account has enough lamports to cover the rent-exempt reserve.

    Accounts expected by this instruction:
      0. `[w]` Metadata
      1. `[]` Update authority
      2. `[]` Mint
      3. `[s]` Mint authority
*/
export interface Initialize {
    programId: PublicKey;
    metadata: PublicKey;
    updateAuthority: PublicKey;
    mint: PublicKey;
    mintAuthority: PublicKey;
    name: string;
    symbol: string;
    uri: string;
}

export function createInitializeInstruction({
    programId,
    metadata,
    updateAuthority,
    mint,
    mintAuthority,
    name,
    symbol,
    uri,
}: Initialize): TransactionInstruction {
    const data = { name, symbol, uri };
    return new TransactionInstruction({
        programId,
        keys: [
            { isSigner: false, isWritable: true, pubkey: metadata },
            { isSigner: false, isWritable: false, pubkey: updateAuthority },
            { isSigner: false, isWritable: false, pubkey: mint },
            { isSigner: true, isWritable: false, pubkey: mintAuthority },
        ],
        data: Buffer.concat([DISCRIMNATOR.Initialize, serialize(SCHEMA.Initialize, data)]),
    });
}

/** If the field does not exist on the account, it will be created.
 * If the field does exist, it will be overwritten. */
interface UpdateFeild {
    programId: PublicKey;
    metadata: PublicKey;
    updateAuthority: PublicKey;
    field: Field;
    value: string;
}

export function createUpdateFieldInstruction({
    programId,
    metadata,
    updateAuthority,
    field,
    value,
}: UpdateFeild): TransactionInstruction {
    const data = { field, value };
    return new TransactionInstruction({
        programId,
        keys: [
            { isSigner: false, isWritable: true, pubkey: metadata },
            { isSigner: true, isWritable: false, pubkey: updateAuthority },
        ],
        data: Buffer.concat([
            DISCRIMNATOR.UpdateField,
            Buffer.from([3]), // TODO explain this. It comes from field being typed as "Field" rather than string
            serialize(SCHEMA.UpdateField, data),
        ]),
    });
}

/** Removes a key-value pair in a token-metadata account. This only applies
 * to additional fields, and not the base name / symbol / URI fields. */
export interface RemoveKey {
    programId: PublicKey;
    metadata: PublicKey;
    updateAuthority: PublicKey;
    field: Field;
    idempotent: boolean;
}

export function createRemoveKeyInstruction({ programId, metadata, updateAuthority, field, idempotent }: RemoveKey) {
    const data = { idempotent, key: field };
    return new TransactionInstruction({
        programId,
        keys: [
            { isSigner: false, isWritable: true, pubkey: metadata },
            { isSigner: true, isWritable: false, pubkey: updateAuthority },
        ],
        data: Buffer.concat([DISCRIMNATOR.RemoveKey, serialize(SCHEMA.RemoveKey, data)]),
    });
}

/** Updates the token-metadata authority */
export interface UpdateAuthority {
    programId: PublicKey;
    metadata: PublicKey;
    oldAuthority: PublicKey;
    newAuthority: PublicKey;
}

export function createUpdateAuthorityInstruction({
    programId,
    metadata,
    oldAuthority,
    newAuthority,
}: UpdateAuthority): TransactionInstruction {
    const data = { newAuthority: newAuthority.toBuffer() };
    return new TransactionInstruction({
        programId,
        keys: [
            { isSigner: false, isWritable: true, pubkey: metadata },
            { isSigner: true, isWritable: false, pubkey: oldAuthority },
        ],
        data: Buffer.concat([DISCRIMNATOR.UpdateAuthorithy, serialize(SCHEMA.UpdateAuthorithy, data)]),
    });
}

// Emits the token-metadata as return data
export interface Emit {
    programId: PublicKey;
    metadata: PublicKey;
    start?: number;
    end?: number;
}

export function createEmitInstruction({ programId, metadata, start, end }: Emit): TransactionInstruction {
    const data = { start, end };
    return new TransactionInstruction({
        programId,
        keys: [{ isSigner: false, isWritable: false, pubkey: metadata }],
        data: Buffer.concat([DISCRIMNATOR.Emit, serialize(SCHEMA.Emit, data)]),
    });
}
