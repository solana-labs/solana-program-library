import type { StructToEncoderTuple } from '@solana/codecs-data-structures';
import { getBooleanEncoder, getBytesEncoder, getDataEnumCodec, getStructEncoder } from '@solana/codecs-data-structures';
import { getU64Encoder } from '@solana/codecs-numbers';
import { getStringEncoder } from '@solana/codecs-strings';
import { getOptionEncoder } from '@solana/options';
import { splDiscriminate } from '@solana/spl-type-length-value';
import type { PublicKey } from '@solana/web3.js';
import { TransactionInstruction } from '@solana/web3.js';

import type { Field } from './field.js';
import { getFieldCodec, getFieldConfig } from './field.js';

function packInstruction<T extends object>(
    layout: StructToEncoderTuple<T>,
    discriminator: Uint8Array,
    values: T
): Buffer {
    const encoder = getStructEncoder(layout);
    const data = encoder.encode(values);
    return Buffer.concat([discriminator, data]);
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
        data: packInstruction(
            [
                ['name', getStringEncoder()],
                ['symbol', getStringEncoder()],
                ['uri', getStringEncoder()],
            ],
            splDiscriminate('spl_token_metadata_interface:initialize_account'),
            { name, symbol, uri }
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
        data: packInstruction(
            [
                ['field', getDataEnumCodec(getFieldCodec())],
                ['value', getStringEncoder()],
            ],
            splDiscriminate('spl_token_metadata_interface:updating_field'),
            { field: getFieldConfig(field), value }
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
        data: packInstruction(
            [
                ['idempotent', getBooleanEncoder()],
                ['key', getStringEncoder()],
            ],
            splDiscriminate('spl_token_metadata_interface:remove_key_ix'),
            { idempotent, key }
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

    const newAuthorityBuffer = Buffer.alloc(32);
    if (newAuthority) {
        newAuthorityBuffer.set(newAuthority.toBuffer());
    } else {
        newAuthorityBuffer.fill(0);
    }

    return new TransactionInstruction({
        programId,
        keys: [
            { isSigner: false, isWritable: true, pubkey: metadata },
            { isSigner: true, isWritable: false, pubkey: oldAuthority },
        ],
        data: packInstruction(
            [['newAuthority', getBytesEncoder({ size: 32 })]],
            splDiscriminate('spl_token_metadata_interface:update_the_authority'),
            { newAuthority: newAuthorityBuffer }
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
        data: packInstruction(
            [
                ['start', getOptionEncoder(getU64Encoder())],
                ['end', getOptionEncoder(getU64Encoder())],
            ],
            splDiscriminate('spl_token_metadata_interface:emitter'),
            { start: start ?? null, end: end ?? null }
        ),
    });
}
