import { PublicKey, TransactionInstruction } from '@solana/web3.js';
import { expect } from 'chai';
import type { Mint } from '../../src';
import {
    TOKEN_2022_PROGRAM_ID,
    createInitializeGroupPointerInstruction,
    createUpdateGroupPointerInstruction,
    getGroupPointerState,
} from '../../src';

const AUTHORITY_ADDRESS_BYTES = Buffer.alloc(32).fill(8);
const GROUP_ADDRESS_BYTES = Buffer.alloc(32).fill(5);
const NULL_OPTIONAL_NONZERO_PUBKEY_BYTES = Buffer.alloc(32).fill(0);

describe('SPL Token 2022 GroupPointer Extension', () => {
    it('can create InitializeGroupPointerInstruction', () => {
        const mint = PublicKey.unique();
        const authority = new PublicKey(AUTHORITY_ADDRESS_BYTES);
        const groupAddress = new PublicKey(GROUP_ADDRESS_BYTES);
        const instruction = createInitializeGroupPointerInstruction(
            mint,
            authority,
            groupAddress,
            TOKEN_2022_PROGRAM_ID,
        );
        expect(instruction).to.deep.equal(
            new TransactionInstruction({
                programId: TOKEN_2022_PROGRAM_ID,
                keys: [{ isSigner: false, isWritable: true, pubkey: mint }],
                data: Buffer.concat([
                    Buffer.from([
                        40, // Token instruction discriminator
                        0, // GroupPointer instruction discriminator
                    ]),
                    AUTHORITY_ADDRESS_BYTES,
                    GROUP_ADDRESS_BYTES,
                ]),
            }),
        );
    });
    it('can create UpdateGroupPointerInstruction', () => {
        const mint = PublicKey.unique();
        const authority = PublicKey.unique();
        const groupAddress = new PublicKey(GROUP_ADDRESS_BYTES);
        const instruction = createUpdateGroupPointerInstruction(mint, authority, groupAddress);
        expect(instruction).to.deep.equal(
            new TransactionInstruction({
                programId: TOKEN_2022_PROGRAM_ID,
                keys: [
                    { isSigner: false, isWritable: true, pubkey: mint },
                    { isSigner: true, isWritable: false, pubkey: authority },
                ],
                data: Buffer.concat([
                    Buffer.from([
                        40, // Token instruction discriminator
                        1, // GroupPointer instruction discriminator
                    ]),
                    GROUP_ADDRESS_BYTES,
                ]),
            }),
        );
    });
    it('can create UpdateGroupPointerInstruction to none', () => {
        const mint = PublicKey.unique();
        const authority = PublicKey.unique();
        const groupAddress = null;
        const instruction = createUpdateGroupPointerInstruction(mint, authority, groupAddress);
        expect(instruction).to.deep.equal(
            new TransactionInstruction({
                programId: TOKEN_2022_PROGRAM_ID,
                keys: [
                    { isSigner: false, isWritable: true, pubkey: mint },
                    { isSigner: true, isWritable: false, pubkey: authority },
                ],
                data: Buffer.concat([
                    Buffer.from([
                        40, // Token instruction discriminator
                        1, // GroupPointer instruction discriminator
                    ]),
                    NULL_OPTIONAL_NONZERO_PUBKEY_BYTES,
                ]),
            }),
        );
    });
    it('can get state with authority and group address', async () => {
        const mintInfo = {
            tlvData: Buffer.concat([
                Buffer.from([
                    // Extension discriminator
                    20, 0,
                    // Extension length
                    64, 0,
                ]),
                AUTHORITY_ADDRESS_BYTES,
                GROUP_ADDRESS_BYTES,
            ]),
        } as Mint;
        const groupPointer = getGroupPointerState(mintInfo);
        expect(groupPointer).to.deep.equal({
            authority: new PublicKey(AUTHORITY_ADDRESS_BYTES),
            groupAddress: new PublicKey(GROUP_ADDRESS_BYTES),
        });
    });
    it('can get state with only group address', async () => {
        const mintInfo = {
            tlvData: Buffer.concat([
                Buffer.from([
                    // Extension discriminator
                    20, 0,
                    // Extension length
                    64, 0,
                ]),
                NULL_OPTIONAL_NONZERO_PUBKEY_BYTES,
                GROUP_ADDRESS_BYTES,
            ]),
        } as Mint;
        const groupPointer = getGroupPointerState(mintInfo);
        expect(groupPointer).to.deep.equal({
            authority: null,
            groupAddress: new PublicKey(GROUP_ADDRESS_BYTES),
        });
    });
    it('can get state with only authority address', async () => {
        const mintInfo = {
            tlvData: Buffer.concat([
                Buffer.from([
                    // Extension discriminator
                    20, 0,
                    // Extension length
                    64, 0,
                ]),
                AUTHORITY_ADDRESS_BYTES,
                NULL_OPTIONAL_NONZERO_PUBKEY_BYTES,
            ]),
        } as Mint;
        const groupPointer = getGroupPointerState(mintInfo);
        expect(groupPointer).to.deep.equal({
            authority: new PublicKey(AUTHORITY_ADDRESS_BYTES),
            groupAddress: null,
        });
    });
});
