import { Keypair, LAMPORTS_PER_SOL, PublicKey, SystemProgram } from '@solana/web3.js';

import {
    createInstruction,
    deleteInstruction,
    reallocInstruction,
    transferInstruction,
    updateInstruction,
} from '../../src';
import { Numberu32, Numberu64 } from '../../src/utils';

import { describe, expect, test } from '@jest/globals';

describe('SplNameService Instructions', () => {
    const nameServiceAddress = new PublicKey('namesLPneVptA9Z5rqUDD9tMTWEJwofgaYwp8cawRkX');
    const nameAccountKey = Keypair.generate().publicKey;
    const nameOwnerKey = Keypair.generate().publicKey;
    const payerKey = Keypair.generate().publicKey;
    const nameClassKey = Keypair.generate().publicKey;
    const nameParent = Keypair.generate().publicKey;
    const nameParentOwner = Keypair.generate().publicKey;
    const name = Buffer.from('hello');

    test('createInstruction without class and parent name key', () => {
        const instruction = createInstruction(
            nameServiceAddress,
            SystemProgram.programId,
            nameAccountKey,
            nameOwnerKey,
            payerKey,
            name,
            new Numberu64(LAMPORTS_PER_SOL),
            new Numberu64(10),
        );

        expect(instruction.keys).toHaveLength(6);
        instruction.keys[0].pubkey.equals(SystemProgram.programId);
        instruction.keys[1].pubkey.equals(payerKey);
        instruction.keys[2].pubkey.equals(nameAccountKey);
        instruction.keys[3].pubkey.equals(nameOwnerKey);
        instruction.keys[4].pubkey.equals(new PublicKey(Buffer.alloc(32)));
        instruction.keys[5].pubkey.equals(new PublicKey(Buffer.alloc(32)));
    });

    test('createInstruction with class and parent name key', () => {
        const instruction = createInstruction(
            nameServiceAddress,
            SystemProgram.programId,
            nameAccountKey,
            nameOwnerKey,
            payerKey,
            name,
            new Numberu64(LAMPORTS_PER_SOL),
            new Numberu64(10),
            nameClassKey,
            nameParent,
            nameParentOwner,
        );

        expect(instruction.keys).toHaveLength(7);
        instruction.keys[0].pubkey.equals(SystemProgram.programId);
        instruction.keys[1].pubkey.equals(payerKey);
        instruction.keys[2].pubkey.equals(nameAccountKey);
        instruction.keys[3].pubkey.equals(nameOwnerKey);
        instruction.keys[4].pubkey.equals(nameClassKey);
        instruction.keys[5].pubkey.equals(nameParent);
        instruction.keys[6].pubkey.equals(nameParentOwner);
    });

    test('updateInstruction', () => {
        const data = Buffer.from('@Dudl');
        const instruction = updateInstruction(
            nameServiceAddress,
            nameAccountKey,
            new Numberu32(0),
            data,
            nameOwnerKey,
            undefined,
        );

        expect(instruction.keys).toHaveLength(2);
        instruction.keys[0].pubkey.equals(nameAccountKey);
        instruction.keys[1].pubkey.equals(nameOwnerKey);
    });

    test('transferInstruction', () => {
        const newOwner = Keypair.generate().publicKey;
        const instruction = transferInstruction(nameServiceAddress, nameAccountKey, newOwner, nameOwnerKey);

        expect(instruction.keys).toHaveLength(2);
        instruction.keys[0].pubkey.equals(nameAccountKey);
        instruction.keys[1].pubkey.equals(nameOwnerKey);
    });

    test('deleteInstruction', () => {
        const instruction = deleteInstruction(nameServiceAddress, nameAccountKey, payerKey, nameOwnerKey);

        expect(instruction.keys).toHaveLength(3);
        instruction.keys[0].pubkey.equals(nameAccountKey);
        instruction.keys[1].pubkey.equals(nameOwnerKey);
        instruction.keys[2].pubkey.equals(payerKey);
    });

    test('reallocInstruction', () => {
        const instruction = reallocInstruction(
            nameServiceAddress,
            SystemProgram.programId,
            payerKey,
            nameAccountKey,
            nameOwnerKey,
            new Numberu32(30),
        );

        expect(instruction.keys).toHaveLength(4);
        instruction.keys[0].pubkey.equals(SystemProgram.programId);
        instruction.keys[1].pubkey.equals(payerKey);
        instruction.keys[2].pubkey.equals(nameAccountKey);
        instruction.keys[3].pubkey.equals(nameOwnerKey);
    });
});
