import type { StructToEncoderTuple } from '@solana/codecs-data-structures';
import type { PublicKey } from '@solana/web3.js';
import { getBytesEncoder, getStructEncoder } from '@solana/codecs-data-structures';
import { getU32Encoder } from '@solana/codecs-numbers';
import { splDiscriminate } from '@solana/spl-type-length-value';
import { TransactionInstruction } from '@solana/web3.js';

function packInstruction<T extends object>(
    layout: StructToEncoderTuple<T>,
    discriminator: Uint8Array,
    values: T
): Buffer {
    const encoder = getStructEncoder(layout);
    const data = encoder.encode(values);
    return Buffer.concat([discriminator, data]);
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

    const updateAuthorityBuffer = Buffer.alloc(32);
    if (updateAuthority) {
        updateAuthorityBuffer.set(updateAuthority.toBuffer());
    } else {
        updateAuthorityBuffer.fill(0);
    }

    return new TransactionInstruction({
        programId,
        keys: [
            { isSigner: false, isWritable: true, pubkey: group },
            { isSigner: false, isWritable: false, pubkey: mint },
            { isSigner: true, isWritable: false, pubkey: mintAuthority },
        ],
        data: packInstruction(
            [
                ['updateAuthority', getBytesEncoder({ size: 32 })],
                ['maxSize', getU32Encoder()],
            ],
            splDiscriminate('spl_token_group_interface:initialize_token_group'),
            { updateAuthority: updateAuthorityBuffer, maxSize }
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
        data: packInstruction(
            [['maxSize', getU32Encoder()]],
            splDiscriminate('spl_token_group_interface:update_group_max_size'),
            { maxSize }
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

    const newAuthorityBuffer = Buffer.alloc(32);
    if (newAuthority) {
        newAuthorityBuffer.set(newAuthority.toBuffer());
    } else {
        newAuthorityBuffer.fill(0);
    }

    return new TransactionInstruction({
        programId,
        keys: [
            { isSigner: false, isWritable: true, pubkey: group },
            { isSigner: true, isWritable: false, pubkey: currentAuthority },
        ],
        data: packInstruction(
            [['newAuthority', getBytesEncoder({ size: 32 })]],
            splDiscriminate('spl_token_group_interface:update_authority'),
            { newAuthority: newAuthorityBuffer }
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
        data: packInstruction([], splDiscriminate('spl_token_group_interface:initialize_member'), {}),
    });
}
