import { PublicKey, TransactionInstruction } from '@solana/web3.js';
import { expect } from 'chai';

import {
    createEmitInstruction,
    createInitializeInstruction,
    createRemoveKeyInstruction,
    createUpdateAuthorityInstruction,
    createUpdateFieldInstruction,
} from '../src';

describe('Token Metadata Instructions', () => {
    const programId = new PublicKey('22222222222222222222222222222222222222222222');
    const metadata = new PublicKey('33333333333333333333333333333333333333333333');
    const updateAuthority = new PublicKey('44444444444444444444444444444444444444444444');
    const mint = new PublicKey('55555555555555555555555555555555555555555555');
    const mintAuthority = new PublicKey('66666666666666666666666666666666666666666666');

    it('Can create Initialize Instruction', () => {
        const instruction = createInitializeInstruction({
            programId,
            metadata,
            updateAuthority,
            mint,
            mintAuthority,
            name: 'My test token',
            symbol: 'TEST',
            uri: 'http://test.test',
        });

        expect(instruction).to.deep.equal(
            new TransactionInstruction({
                programId,
                keys: [
                    { isSigner: false, isWritable: true, pubkey: metadata },
                    { isSigner: false, isWritable: false, pubkey: updateAuthority },
                    { isSigner: false, isWritable: false, pubkey: mint },
                    { isSigner: true, isWritable: false, pubkey: mintAuthority },
                ],
                data: Buffer.from([
                    // Output of rust implementation
                    210, 225, 30, 162, 88, 184, 77, 141, 13, 0, 0, 0, 77, 121, 32, 116, 101, 115, 116, 32, 116, 111,
                    107, 101, 110, 4, 0, 0, 0, 84, 69, 83, 84, 16, 0, 0, 0, 104, 116, 116, 112, 58, 47, 47, 116, 101,
                    115, 116, 46, 116, 101, 115, 116,
                ]),
            })
        );
    });

    it('Can create Update Field Instruction', () => {
        const instruction = createUpdateFieldInstruction({
            programId,
            metadata,
            updateAuthority,
            field: 'MyTestField',
            value: 'http://test.uri',
        });

        expect(instruction).to.deep.equal(
            new TransactionInstruction({
                programId,
                keys: [
                    { isSigner: false, isWritable: true, pubkey: metadata },
                    { isSigner: true, isWritable: false, pubkey: updateAuthority },
                ],
                data: Buffer.from([
                    // Output of rust implementation
                    221, 233, 49, 45, 181, 202, 220, 200, 3, 11, 0, 0, 0, 77, 121, 84, 101, 115, 116, 70, 105, 101, 108,
                    100, 15, 0, 0, 0, 104, 116, 116, 112, 58, 47, 47, 116, 101, 115, 116, 46, 117, 114, 105,
                ]),
            })
        );
    });

    it('Can create Update Field Instruction with Field Enum', () => {
        const instruction = createUpdateFieldInstruction({
            programId,
            metadata,
            updateAuthority,
            field: 'Name',
            value: 'http://test.uri',
        });

        expect(instruction).to.deep.equal(
            new TransactionInstruction({
                programId,
                keys: [
                    { isSigner: false, isWritable: true, pubkey: metadata },
                    { isSigner: true, isWritable: false, pubkey: updateAuthority },
                ],
                data: Buffer.from([
                    // Output of rust implementation
                    221, 233, 49, 45, 181, 202, 220, 200, 0, 15, 0, 0, 0, 104, 116, 116, 112, 58, 47, 47, 116, 101, 115,
                    116, 46, 117, 114, 105,
                ]),
            })
        );
    });

    it('Can create Remove Key Instruction', () => {
        const instruction = createRemoveKeyInstruction({
            programId,
            metadata,
            updateAuthority: updateAuthority,
            key: 'MyTestField',
            idempotent: true,
        });

        expect(instruction).to.deep.equal(
            new TransactionInstruction({
                programId,
                keys: [
                    { isSigner: false, isWritable: true, pubkey: metadata },
                    { isSigner: true, isWritable: false, pubkey: updateAuthority },
                ],
                data: Buffer.from([
                    // Output of rust implementation
                    234, 18, 32, 56, 89, 141, 37, 181, 1, 11, 0, 0, 0, 77, 121, 84, 101, 115, 116, 70, 105, 101, 108,
                    100,
                ]),
            })
        );
    });

    it('Can create Update Authority Instruction', () => {
        const instruction = createUpdateAuthorityInstruction({
            programId,
            metadata,
            oldAuthority: updateAuthority,
            newAuthority: PublicKey.default,
        });

        expect(instruction).to.deep.equal(
            new TransactionInstruction({
                programId,
                keys: [
                    { isSigner: false, isWritable: true, pubkey: metadata },
                    { isSigner: true, isWritable: false, pubkey: updateAuthority },
                ],
                data: Buffer.from([
                    // Output of rust implementation
                    215, 228, 166, 228, 84, 100, 86, 123, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                ]),
            })
        );
    });
    it('Can create Emit Instruction', () => {
        const instruction = createEmitInstruction({
            programId,
            metadata,
            end: BigInt(10),
        });

        expect(instruction).to.deep.equal(
            new TransactionInstruction({
                programId,
                keys: [{ isSigner: false, isWritable: false, pubkey: metadata }],
                data: Buffer.from([
                    // Output of rust implementation
                    250, 166, 180, 250, 13, 12, 184, 70, 0, 1, 10, 0, 0, 0, 0, 0, 0, 0,
                ]),
            })
        );
    });
});
