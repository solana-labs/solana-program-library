import { PublicKey, TransactionInstruction } from '@solana/web3.js';
import { expect } from 'chai';
import type { Mint } from '../../src';
import {
    TOKEN_2022_PROGRAM_ID,
    createInitializeMetadataPointerInstruction,
    createUpdateMetadataPointerInstruction,
    getMetadataPointerState,
} from '../../src';

describe('SPL Token 2022 Metadata Extension', () => {
    it('can create createInitializeMetadataPointerInstruction', () => {
        const mint = PublicKey.unique();
        const authority = new PublicKey('1111111QLbz7JHiBTspS962RLKV8GndWFwiEaqKM');
        const metadataAddress = new PublicKey('1111111ogCyDbaRMvkdsHB3qfdyFYaG1WtRUAfdh');

        const instruction = createInitializeMetadataPointerInstruction(
            mint,
            authority,
            metadataAddress,
            TOKEN_2022_PROGRAM_ID
        );

        expect(instruction).to.deep.equal(
            new TransactionInstruction({
                programId: TOKEN_2022_PROGRAM_ID,
                keys: [{ isSigner: false, isWritable: true, pubkey: mint }],
                data: Buffer.from([
                    // Output of rust implementation
                    39, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                ]),
            })
        );
    });

    it('can create createUpdateMetadataPointerInstruction', () => {
        const mint = PublicKey.unique();
        const authority = PublicKey.unique();
        const metadataAddress = new PublicKey('11111112cMQwSC9qirWGjZM6gLGwW69X22mqwLLGP');

        const instruction = createUpdateMetadataPointerInstruction(mint, authority, metadataAddress);

        expect(instruction).to.deep.equal(
            new TransactionInstruction({
                programId: TOKEN_2022_PROGRAM_ID,
                keys: [
                    { isSigner: false, isWritable: true, pubkey: mint },
                    { isSigner: true, isWritable: false, pubkey: authority },
                ],
                data: Buffer.from([
                    // Output of rust implementation
                    39, 1, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0,
                ]),
            })
        );
    });

    it('can create createUpdateMetadataPointerInstruction to none', () => {
        const mint = PublicKey.unique();
        const authority = PublicKey.unique();
        const metadataAddress = null;

        const instruction = createUpdateMetadataPointerInstruction(mint, authority, metadataAddress);

        expect(instruction).to.deep.equal(
            new TransactionInstruction({
                programId: TOKEN_2022_PROGRAM_ID,
                keys: [
                    { isSigner: false, isWritable: true, pubkey: mint },
                    { isSigner: true, isWritable: false, pubkey: authority },
                ],
                data: Buffer.from([
                    // Output of rust implementation
                    39, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0,
                ]),
            })
        );
    });

    it('can get state with authority and metadata address', async () => {
        const mintInfo = {
            tlvData: Buffer.from([
                18, 0, 64, 0, 134, 125, 9, 16, 205, 223, 26, 224, 220, 174, 52, 213, 193, 216, 9, 80, 82, 181, 8, 228,
                75, 112, 233, 116, 2, 183, 51, 228, 88, 64, 179, 158, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1,
            ]),
        } as Mint;

        const metadataPointer = getMetadataPointerState(mintInfo);

        expect(metadataPointer).to.deep.equal({
            authority: new PublicKey([
                134, 125, 9, 16, 205, 223, 26, 224, 220, 174, 52, 213, 193, 216, 9, 80, 82, 181, 8, 228, 75, 112, 233,
                116, 2, 183, 51, 228, 88, 64, 179, 158,
            ]),
            metadataAddress: new PublicKey([
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1,
            ]),
        });
    });
    it('can get state with only metadata address', async () => {
        const mintInfo = {
            tlvData: Buffer.from([
                18, 0, 64, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1,
            ]),
        } as Mint;

        const metadataPointer = getMetadataPointerState(mintInfo);

        expect(metadataPointer).to.deep.equal({
            authority: null,
            metadataAddress: new PublicKey([
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1,
            ]),
        });
    });

    it('can get state with only authority address', async () => {
        const mintInfo = {
            tlvData: Buffer.from([
                18, 0, 64, 0, 16, 218, 238, 42, 17, 19, 152, 173, 216, 24, 229, 204, 215, 108, 49, 98, 233, 115, 53,
                252, 9, 156, 216, 23, 14, 157, 139, 132, 28, 182, 4, 191, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            ]),
        } as Mint;

        const metadataPointer = getMetadataPointerState(mintInfo);

        expect(metadataPointer).to.deep.equal({
            authority: new PublicKey([
                16, 218, 238, 42, 17, 19, 152, 173, 216, 24, 229, 204, 215, 108, 49, 98, 233, 115, 53, 252, 9, 156, 216,
                23, 14, 157, 139, 132, 28, 182, 4, 191,
            ]),
            metadataAddress: null,
        });
    });
});
