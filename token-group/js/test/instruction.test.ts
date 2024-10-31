import { expect } from 'chai';
import type { Decoder } from '@solana/codecs';
import { fixDecoderSize, getBytesDecoder, getStructDecoder, getU64Decoder } from '@solana/codecs';
import { splDiscriminate } from '@solana/spl-type-length-value';
import { PublicKey, type TransactionInstruction } from '@solana/web3.js';

import {
    createInitializeGroupInstruction,
    createInitializeMemberInstruction,
    createUpdateGroupMaxSizeInstruction,
    createUpdateGroupAuthorityInstruction,
} from '../src';

function checkPackUnpack<T extends object>(
    instruction: TransactionInstruction,
    discriminator: Uint8Array,
    decoder: Decoder<T>,
    values: T,
) {
    expect(instruction.data.subarray(0, 8)).to.deep.equal(discriminator);
    const unpacked = decoder.decode(instruction.data.subarray(8));
    expect(unpacked).to.deep.equal(values);
}

describe('Token Group Instructions', () => {
    const programId = new PublicKey('22222222222222222222222222222222222222222222');
    const group = new PublicKey('33333333333333333333333333333333333333333333');
    const updateAuthority = new PublicKey('44444444444444444444444444444444444444444444');
    const mint = new PublicKey('55555555555555555555555555555555555555555555');
    const mintAuthority = new PublicKey('66666666666666666666666666666666666666666666');
    const maxSize = BigInt(100);

    it('Can create InitializeGroup Instruction', async () => {
        checkPackUnpack(
            createInitializeGroupInstruction({
                programId,
                group,
                mint,
                mintAuthority,
                updateAuthority,
                maxSize,
            }),
            await splDiscriminate('spl_token_group_interface:initialize_token_group'),
            getStructDecoder([
                ['updateAuthority', fixDecoderSize(getBytesDecoder(), 32)],
                ['maxSize', getU64Decoder()],
            ]),
            { updateAuthority: Uint8Array.from(updateAuthority.toBuffer()), maxSize },
        );
    });

    it('Can create UpdateGroupMaxSize Instruction', async () => {
        checkPackUnpack(
            createUpdateGroupMaxSizeInstruction({
                programId,
                group,
                updateAuthority,
                maxSize,
            }),
            await splDiscriminate('spl_token_group_interface:update_group_max_size'),
            getStructDecoder([['maxSize', getU64Decoder()]]),
            { maxSize },
        );
    });

    it('Can create UpdateGroupAuthority Instruction', async () => {
        checkPackUnpack(
            createUpdateGroupAuthorityInstruction({
                programId,
                group,
                currentAuthority: updateAuthority,
                newAuthority: PublicKey.default,
            }),
            await splDiscriminate('spl_token_group_interface:update_authority'),
            getStructDecoder([['newAuthority', fixDecoderSize(getBytesDecoder(), 32)]]),
            { newAuthority: Uint8Array.from(PublicKey.default.toBuffer()) },
        );
    });

    it('Can create InitializeMember Instruction', async () => {
        const member = new PublicKey('22222222222222222222222222222222222222222222');
        const memberMint = new PublicKey('33333333333333333333333333333333333333333333');
        const memberMintAuthority = new PublicKey('44444444444444444444444444444444444444444444');
        const group = new PublicKey('55555555555555555555555555555555555555555555');
        const groupUpdateAuthority = new PublicKey('66666666666666666666666666666666666666666666');

        checkPackUnpack(
            createInitializeMemberInstruction({
                programId,
                member,
                memberMint,
                memberMintAuthority,
                group,
                groupUpdateAuthority,
            }),
            await splDiscriminate('spl_token_group_interface:initialize_member'),
            getStructDecoder([]),
            {},
        );
    });
});
