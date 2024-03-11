import type { Encoder } from '@solana/codecs';
import {
    getBooleanEncoder,
    getBytesEncoder,
    getDataEnumCodec,
    getOptionEncoder,
    getStringEncoder,
    getStructEncoder,
    getTupleEncoder,
    getU64Encoder,
    mapEncoder,
} from '@solana/codecs';
import { splDiscriminate } from '@solana/spl-type-length-value';
import type { PublicKey } from '@solana/web3.js';
import { SystemProgram, TransactionInstruction } from '@solana/web3.js';

import type { Field } from './field.js';
import { getFieldCodec, getFieldConfig } from './field.js';

function getInstructionEncoder<T extends object>(discriminator: Uint8Array, dataEncoder: Encoder<T>): Encoder<T> {
    return mapEncoder(getTupleEncoder([getBytesEncoder(), dataEncoder]), (data: T): [Uint8Array, T] => [
        discriminator,
        data,
    ]);
}

function getPublicKeyEncoder(): Encoder<PublicKey> {
    return mapEncoder(getBytesEncoder({ size: 32 }), (publicKey: PublicKey) => publicKey.toBytes());
}

/**
 * Initializes a TLV entry with the basic token-metadata fields.
 *
 * Assumes that the provided mint is an SPL token mint, that the metadata
 * account is allocated and assigned to the program, and that the metadata
 * account has enough lamports to cover the rent-exempt reserve.
 */
export interface InitializeInstructionArgs {
    programId: PublicKey;
    metadata: PublicKey;
    updateAuthority: PublicKey;
    mint: PublicKey;
    mintAuthority: PublicKey;
    name: string;
    symbol: string;
    uri: string;
}

export function createInitializeInstruction(args: InitializeInstructionArgs): TransactionInstruction {
    const { programId, metadata, updateAuthority, mint, mintAuthority, name, symbol, uri } = args;
    return new TransactionInstruction({
        programId,
        keys: [
            { isSigner: false, isWritable: true, pubkey: metadata },
            { isSigner: false, isWritable: false, pubkey: updateAuthority },
            { isSigner: false, isWritable: false, pubkey: mint },
            { isSigner: true, isWritable: false, pubkey: mintAuthority },
        ],
        data: Buffer.from(
            getInstructionEncoder(
                splDiscriminate('spl_token_metadata_interface:initialize_account'),
                getStructEncoder([
                    ['name', getStringEncoder()],
                    ['symbol', getStringEncoder()],
                    ['uri', getStringEncoder()],
                ])
            ).encode({ name, symbol, uri })
        ),
    });
}

/**
 * If the field does not exist on the account, it will be created.
 * If the field does exist, it will be overwritten.
 */
export interface UpdateFieldInstruction {
    programId: PublicKey;
    metadata: PublicKey;
    updateAuthority: PublicKey;
    field: Field | string;
    value: string;
}

export function createUpdateFieldInstruction(args: UpdateFieldInstruction): TransactionInstruction {
    const { programId, metadata, updateAuthority, field, value } = args;
    return new TransactionInstruction({
        programId,
        keys: [
            { isSigner: false, isWritable: true, pubkey: metadata },
            { isSigner: true, isWritable: false, pubkey: updateAuthority },
        ],
        data: Buffer.from(
            getInstructionEncoder(
                splDiscriminate('spl_token_metadata_interface:updating_field'),
                getStructEncoder([
                    ['field', getDataEnumCodec(getFieldCodec())],
                    ['value', getStringEncoder()],
                ])
            ).encode({ field: getFieldConfig(field), value })
        ),
    });
}

export interface RemoveKeyInstructionArgs {
    programId: PublicKey;
    metadata: PublicKey;
    updateAuthority: PublicKey;
    key: string;
    idempotent: boolean;
}

export function createRemoveKeyInstruction(args: RemoveKeyInstructionArgs) {
    const { programId, metadata, updateAuthority, key, idempotent } = args;
    return new TransactionInstruction({
        programId,
        keys: [
            { isSigner: false, isWritable: true, pubkey: metadata },
            { isSigner: true, isWritable: false, pubkey: updateAuthority },
        ],
        data: Buffer.from(
            getInstructionEncoder(
                splDiscriminate('spl_token_metadata_interface:remove_key_ix'),
                getStructEncoder([
                    ['idempotent', getBooleanEncoder()],
                    ['key', getStringEncoder()],
                ])
            ).encode({ idempotent, key })
        ),
    });
}

export interface UpdateAuthorityInstructionArgs {
    programId: PublicKey;
    metadata: PublicKey;
    oldAuthority: PublicKey;
    newAuthority: PublicKey | null;
}

export function createUpdateAuthorityInstruction(args: UpdateAuthorityInstructionArgs): TransactionInstruction {
    const { programId, metadata, oldAuthority, newAuthority } = args;

    return new TransactionInstruction({
        programId,
        keys: [
            { isSigner: false, isWritable: true, pubkey: metadata },
            { isSigner: true, isWritable: false, pubkey: oldAuthority },
        ],
        data: Buffer.from(
            getInstructionEncoder(
                splDiscriminate('spl_token_metadata_interface:update_the_authority'),
                getStructEncoder([['newAuthority', getPublicKeyEncoder()]])
            ).encode({ newAuthority: newAuthority ?? SystemProgram.programId })
        ),
    });
}

export interface EmitInstructionArgs {
    programId: PublicKey;
    metadata: PublicKey;
    start?: bigint;
    end?: bigint;
}

export function createEmitInstruction(args: EmitInstructionArgs): TransactionInstruction {
    const { programId, metadata, start, end } = args;
    return new TransactionInstruction({
        programId,
        keys: [{ isSigner: false, isWritable: false, pubkey: metadata }],
        data: Buffer.from(
            getInstructionEncoder(
                splDiscriminate('spl_token_metadata_interface:emitter'),
                getStructEncoder([
                    ['start', getOptionEncoder(getU64Encoder())],
                    ['end', getOptionEncoder(getU64Encoder())],
                ])
            ).encode({ start: start ?? null, end: end ?? null })
        ),
    });
}
