import { expect } from 'chai';

import {
    createEmitInstruction,
    createInitializeInstruction,
    createRemoveKeyInstruction,
    createUpdateAuthorityInstruction,
    createUpdateFieldInstruction,
    getFieldCodec,
    getFieldConfig,
} from '../src';
import {
    addDecoderSizePrefix,
    fixDecoderSize,
    getBooleanDecoder,
    getBytesDecoder,
    getDataEnumCodec,
    getOptionDecoder,
    getUtf8Decoder,
    getU32Decoder,
    getU64Decoder,
    getStructDecoder,
    some,
} from '@solana/codecs';
import { splDiscriminate } from '@solana/spl-type-length-value';
import type { Decoder, Option, VariableSizeDecoder } from '@solana/codecs';
import { PublicKey, type TransactionInstruction } from '@solana/web3.js';

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

function getStringDecoder(): VariableSizeDecoder<string> {
    return addDecoderSizePrefix(getUtf8Decoder(), getU32Decoder());
}

describe('Token Metadata Instructions', () => {
    const programId = new PublicKey('22222222222222222222222222222222222222222222');
    const metadata = new PublicKey('33333333333333333333333333333333333333333333');
    const updateAuthority = new PublicKey('44444444444444444444444444444444444444444444');
    const mint = new PublicKey('55555555555555555555555555555555555555555555');
    const mintAuthority = new PublicKey('66666666666666666666666666666666666666666666');

    it('Can create Initialize Instruction', async () => {
        const name = 'My test token';
        const symbol = 'TEST';
        const uri = 'http://test.test';
        checkPackUnpack(
            createInitializeInstruction({
                programId,
                metadata,
                updateAuthority,
                mint,
                mintAuthority,
                name,
                symbol,
                uri,
            }),
            await splDiscriminate('spl_token_metadata_interface:initialize_account'),
            getStructDecoder([
                ['name', getStringDecoder()],
                ['symbol', getStringDecoder()],
                ['uri', getStringDecoder()],
            ]),
            { name, symbol, uri },
        );
    });

    it('Can create Update Field Instruction', async () => {
        const field = 'MyTestField';
        const value = 'http://test.uri';
        checkPackUnpack(
            createUpdateFieldInstruction({
                programId,
                metadata,
                updateAuthority,
                field,
                value,
            }),
            await splDiscriminate('spl_token_metadata_interface:updating_field'),
            getStructDecoder([
                ['key', getDataEnumCodec(getFieldCodec())],
                ['value', getStringDecoder()],
            ]),
            { key: getFieldConfig(field), value },
        );
    });

    it('Can create Update Field Instruction with Field Enum', async () => {
        const field = 'Name';
        const value = 'http://test.uri';
        checkPackUnpack(
            createUpdateFieldInstruction({
                programId,
                metadata,
                updateAuthority,
                field,
                value,
            }),
            await splDiscriminate('spl_token_metadata_interface:updating_field'),
            getStructDecoder([
                ['key', getDataEnumCodec(getFieldCodec())],
                ['value', getStringDecoder()],
            ]),
            { key: getFieldConfig(field), value },
        );
    });

    it('Can create Remove Key Instruction', async () => {
        checkPackUnpack(
            createRemoveKeyInstruction({
                programId,
                metadata,
                updateAuthority: updateAuthority,
                key: 'MyTestField',
                idempotent: true,
            }),
            await splDiscriminate('spl_token_metadata_interface:remove_key_ix'),
            getStructDecoder([
                ['idempotent', getBooleanDecoder()],
                ['key', getStringDecoder()],
            ]),
            { idempotent: true, key: 'MyTestField' },
        );
    });

    it('Can create Update Authority Instruction', async () => {
        const newAuthority = PublicKey.default;
        checkPackUnpack(
            createUpdateAuthorityInstruction({
                programId,
                metadata,
                oldAuthority: updateAuthority,
                newAuthority,
            }),
            await splDiscriminate('spl_token_metadata_interface:update_the_authority'),
            getStructDecoder([['newAuthority', fixDecoderSize(getBytesDecoder(), 32)]]),
            { newAuthority: Uint8Array.from(newAuthority.toBuffer()) },
        );
    });

    it('Can create Emit Instruction', async () => {
        const start: Option<bigint> = some(0n);
        const end: Option<bigint> = some(10n);
        checkPackUnpack(
            createEmitInstruction({
                programId,
                metadata,
                start: 0n,
                end: 10n,
            }),
            await splDiscriminate('spl_token_metadata_interface:emitter'),
            getStructDecoder([
                ['start', getOptionDecoder(getU64Decoder())],
                ['end', getOptionDecoder(getU64Decoder())],
            ]),
            { start, end },
        );
    });
});
