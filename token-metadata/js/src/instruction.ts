import type { PublicKey } from '@solana/web3.js';
import { TransactionInstruction } from '@solana/web3.js';
import { splDiscriminate } from '@solana/spl-type-length-value';
import type { Schema } from 'borsh';
import { serialize } from 'borsh';

import type { Field } from './state.js';

// Values from https://github.com/solana-labs/solana-program-library/blob/master/token-metadata/interface/src/instruction.rs
interface TokenMetadataInstructionType<T> {
    discriminator: Uint8Array;
    layout: Schema;
    pack: (values: T) => Buffer;
}

const Initialize: TokenMetadataInstructionType<{
    name: string;
    symbol: string;
    uri: string;
}> = {
    discriminator: splDiscriminate('spl_token_metadata_interface:initialize_account'),
    layout: { struct: { name: 'string', symbol: 'string', uri: 'string' } },
    pack: (values) => Buffer.concat([Initialize.discriminator, serialize(Initialize.layout, values)]),
};

const UpdateField: TokenMetadataInstructionType<{
    field: Field;
    value: string;
}> = {
    discriminator: splDiscriminate('spl_token_metadata_interface:updating_field'),
    layout: { struct: { field: 'string', value: 'string' } },
    pack: (values) =>
        Buffer.concat([
            UpdateField.discriminator,
            Buffer.from([3]), // TODO explain this. It comes from field being typed as "Field" rather than string
            serialize(UpdateField.layout, values),
        ]),
};

const RemoveKey: TokenMetadataInstructionType<{
    field: Field;
    idempotent: boolean;
}> = {
    discriminator: splDiscriminate('spl_token_metadata_interface:remove_key_ix'),
    layout: { struct: { idempotent: 'bool', field: 'string' } },
    pack: (values) => Buffer.concat([RemoveKey.discriminator, serialize(RemoveKey.layout, values)]),
};

const UpdateAuthority: TokenMetadataInstructionType<{
    newAuthority: PublicKey;
}> = {
    discriminator: splDiscriminate('spl_token_metadata_interface:update_the_authority'),
    layout: { struct: { newAuthority: { array: { type: 'u8', len: 32 } } } },
    pack: ({ newAuthority }) =>
        Buffer.concat([
            UpdateAuthority.discriminator,
            serialize(UpdateAuthority.layout, { newAuthority: newAuthority.toBuffer() }),
        ]),
};

const Emit: TokenMetadataInstructionType<{
    start?: bigint;
    end?: bigint;
}> = {
    discriminator: splDiscriminate('spl_token_metadata_interface:emitter'),
    layout: { struct: { start: { option: 'u64' }, end: { option: 'u64' } } },
    pack: (values) => Buffer.concat([Emit.discriminator, serialize(Emit.layout, values)]),
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
    const values = { name, symbol, uri };
    return new TransactionInstruction({
        programId,
        keys: [
            { isSigner: false, isWritable: true, pubkey: metadata },
            { isSigner: false, isWritable: false, pubkey: updateAuthority },
            { isSigner: false, isWritable: false, pubkey: mint },
            { isSigner: true, isWritable: false, pubkey: mintAuthority },
        ],
        data: Initialize.pack(values),
    });
}

/** If the field does not exist on the account, it will be created.
 * If the field does exist, it will be overwritten. */
interface UpdateField {
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
}: UpdateField): TransactionInstruction {
    const values = { field, value };
    return new TransactionInstruction({
        programId,
        keys: [
            { isSigner: false, isWritable: true, pubkey: metadata },
            { isSigner: true, isWritable: false, pubkey: updateAuthority },
        ],
        data: UpdateField.pack(values),
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
    const values = { idempotent, field };
    return new TransactionInstruction({
        programId,
        keys: [
            { isSigner: false, isWritable: true, pubkey: metadata },
            { isSigner: true, isWritable: false, pubkey: updateAuthority },
        ],
        data: RemoveKey.pack(values),
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
    const values = { newAuthority };
    return new TransactionInstruction({
        programId,
        keys: [
            { isSigner: false, isWritable: true, pubkey: metadata },
            { isSigner: true, isWritable: false, pubkey: oldAuthority },
        ],
        data: UpdateAuthority.pack(values),
    });
}

// Emits the token-metadata as return data
export interface Emit {
    programId: PublicKey;
    metadata: PublicKey;
    start?: bigint;
    end?: bigint;
}

export function createEmitInstruction({ programId, metadata, start, end }: Emit): TransactionInstruction {
    const values = { start, end };
    return new TransactionInstruction({
        programId,
        keys: [{ isSigner: false, isWritable: false, pubkey: metadata }],
        data: Emit.pack(values),
    });
}
