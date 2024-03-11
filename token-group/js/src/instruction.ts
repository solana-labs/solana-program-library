import type { Encoder } from '@solana/codecs';
import type { PublicKey } from '@solana/web3.js';
import { getBytesEncoder, getStructEncoder, getTupleEncoder, getU32Encoder, mapEncoder } from '@solana/codecs';
import { splDiscriminate } from '@solana/spl-type-length-value';
import { SystemProgram, TransactionInstruction } from '@solana/web3.js';

function getInstructionEncoder<T extends object>(discriminator: Uint8Array, dataEncoder: Encoder<T>): Encoder<T> {
    return mapEncoder(getTupleEncoder([getBytesEncoder(), dataEncoder]), (data: T): [Uint8Array, T] => [
        discriminator,
        data,
    ]);
}

function getPublicKeyEncoder(): Encoder<PublicKey> {
    return mapEncoder(getBytesEncoder({ size: 32 }), (publicKey: PublicKey) => publicKey.toBytes());
}

export interface InitializeGroupInstruction {
    programId: PublicKey;
    group: PublicKey;
    mint: PublicKey;
    mintAuthority: PublicKey;
    updateAuthority: PublicKey | null;
    maxSize: number;
}

export function createInitializeGroupInstruction(args: InitializeGroupInstruction): TransactionInstruction {
    const { programId, group, mint, mintAuthority, updateAuthority, maxSize } = args;

    return new TransactionInstruction({
        programId,
        keys: [
            { isSigner: false, isWritable: true, pubkey: group },
            { isSigner: false, isWritable: false, pubkey: mint },
            { isSigner: true, isWritable: false, pubkey: mintAuthority },
        ],
        data: Buffer.from(
            getInstructionEncoder(
                splDiscriminate('spl_token_group_interface:initialize_token_group'),
                getStructEncoder([
                    ['updateAuthority', getPublicKeyEncoder()],
                    ['maxSize', getU32Encoder()],
                ])
            ).encode({ updateAuthority: updateAuthority ?? SystemProgram.programId, maxSize })
        ),
    });
}

export interface UpdateGroupMaxSize {
    programId: PublicKey;
    group: PublicKey;
    updateAuthority: PublicKey;
    maxSize: number;
}

export function createUpdateGroupMaxSizeInstruction(args: UpdateGroupMaxSize): TransactionInstruction {
    const { programId, group, updateAuthority, maxSize } = args;
    return new TransactionInstruction({
        programId,
        keys: [
            { isSigner: false, isWritable: true, pubkey: group },
            { isSigner: true, isWritable: false, pubkey: updateAuthority },
        ],
        data: Buffer.from(
            getInstructionEncoder(
                splDiscriminate('spl_token_group_interface:update_group_max_size'),
                getStructEncoder([['maxSize', getU32Encoder()]])
            ).encode({ maxSize })
        ),
    });
}

export interface UpdateGroupAuthority {
    programId: PublicKey;
    group: PublicKey;
    currentAuthority: PublicKey;
    newAuthority: PublicKey | null;
}

export function createUpdateGroupAuthorityInstruction(args: UpdateGroupAuthority): TransactionInstruction {
    const { programId, group, currentAuthority, newAuthority } = args;

    return new TransactionInstruction({
        programId,
        keys: [
            { isSigner: false, isWritable: true, pubkey: group },
            { isSigner: true, isWritable: false, pubkey: currentAuthority },
        ],
        data: Buffer.from(
            getInstructionEncoder(
                splDiscriminate('spl_token_group_interface:update_authority'),
                getStructEncoder([['newAuthority', getPublicKeyEncoder()]])
            ).encode({ newAuthority: newAuthority ?? SystemProgram.programId })
        ),
    });
}

export interface InitializeMember {
    programId: PublicKey;
    member: PublicKey;
    memberMint: PublicKey;
    memberMintAuthority: PublicKey;
    group: PublicKey;
    groupUpdateAuthority: PublicKey;
}

export function createInitializeMemberInstruction(args: InitializeMember): TransactionInstruction {
    const { programId, member, memberMint, memberMintAuthority, group, groupUpdateAuthority } = args;

    return new TransactionInstruction({
        programId,
        keys: [
            { isSigner: false, isWritable: true, pubkey: member },
            { isSigner: false, isWritable: false, pubkey: memberMint },
            { isSigner: true, isWritable: false, pubkey: memberMintAuthority },
            { isSigner: false, isWritable: true, pubkey: group },
            { isSigner: true, isWritable: false, pubkey: groupUpdateAuthority },
        ],
        data: Buffer.from(
            getInstructionEncoder(
                splDiscriminate('spl_token_group_interface:initialize_member'),
                getStructEncoder([])
            ).encode({})
        ),
    });
}
