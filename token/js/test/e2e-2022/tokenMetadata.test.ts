import { expect } from 'chai';

import type { Connection, Signer } from '@solana/web3.js';
import { sendAndConfirmTransaction, PublicKey, Keypair, SystemProgram, Transaction } from '@solana/web3.js';

import {
    ExtensionType,
    createInitializeMetadataPointerInstruction,
    createInitializeMintInstruction,
    tokenMetadataEmit,
    tokenMetadataInitialize,
    tokenMetadataUpdateField,
    getMintLen,
    getTokenMetadata,
    getEmittedTokenMetadata,
} from '../../src';
import { TEST_PROGRAM_ID, newAccountWithLamports, getConnection } from '../common';

const TEST_TOKEN_DECIMALS = 2;

describe('Token Metadata', () => {
    let connection: Connection;
    let payer: Signer;
    let mint: Keypair;
    const authority = Keypair.fromSecretKey(
        new Uint8Array([
            118, 177, 37, 231, 15, 88, 210, 92, 79, 231, 202, 22, 11, 15, 121, 54, 95, 229, 149, 119, 48, 177, 187, 198,
            223, 51, 225, 74, 12, 54, 172, 36, 207, 107, 122, 208, 209, 168, 61, 177, 190, 137, 23, 156, 84, 32, 34, 82,
            158, 176, 55, 51, 236, 66, 130, 167, 118, 31, 120, 107, 100, 192, 147, 10,
        ])
    );

    before(async () => {
        connection = await getConnection();
        payer = await newAccountWithLamports(connection, 1000000000);
    });

    beforeEach(async () => {
        mint = Keypair.generate();

        const EXTENSIONS = [ExtensionType.MetadataPointer];
        const mintLen = getMintLen(EXTENSIONS);

        const lamports = await connection.getMinimumBalanceForRentExemption(mintLen);

        const transaction = new Transaction().add(
            SystemProgram.createAccount({
                fromPubkey: payer.publicKey,
                newAccountPubkey: mint.publicKey,
                space: mintLen,
                lamports: lamports * 10, //TODO:- Handle rent
                programId: TEST_PROGRAM_ID,
            }),
            createInitializeMetadataPointerInstruction(
                mint.publicKey,
                authority.publicKey,
                mint.publicKey,
                TEST_PROGRAM_ID
            ),
            createInitializeMintInstruction(
                mint.publicKey,
                TEST_TOKEN_DECIMALS,
                authority.publicKey,
                null,
                TEST_PROGRAM_ID
            )
        );

        await sendAndConfirmTransaction(connection, transaction, [payer, mint], undefined);

        await tokenMetadataInitialize(
            connection,
            payer,
            authority.publicKey,
            mint.publicKey,
            authority,
            'name',
            'symbol',
            'uri',
            undefined,
            undefined,
            TEST_PROGRAM_ID
        );
    });

    it('can successfully initialize', async () => {
        const meta = await getTokenMetadata(connection, mint.publicKey, undefined, TEST_PROGRAM_ID);
        expect(meta).to.deep.equal({
            updateAuthority: new PublicKey('ExgT3gCWXJzY4a9SHqTqsTk6dPAj37WNq2uWNbmMG1JR'),
            mint: mint.publicKey,
            name: 'name',
            symbol: 'symbol',
            uri: 'uri',
            additionalMetadata: [],
        });
    });

    it('can successfully emit', async () => {
        const signature = await tokenMetadataEmit(connection, payer, mint.publicKey);

        const meta = await getEmittedTokenMetadata(connection, signature);

        expect(meta).to.deep.equal({
            updateAuthority: new PublicKey('ExgT3gCWXJzY4a9SHqTqsTk6dPAj37WNq2uWNbmMG1JR'),
            mint: mint.publicKey,
            name: 'name',
            symbol: 'symbol',
            uri: 'uri',
            additionalMetadata: [],
        });
    });

    it('can successfully update', async () => {
        await Promise.all([
            tokenMetadataUpdateField(
                connection,
                payer,
                authority,
                mint.publicKey,
                'TVL',
                '1,000,000',
                undefined,
                undefined,
                TEST_PROGRAM_ID
            ),
            tokenMetadataUpdateField(
                connection,
                payer,
                authority,
                mint.publicKey,
                'name',
                'TEST',
                undefined,
                undefined,
                TEST_PROGRAM_ID
            ),
        ]);

        const meta = await getTokenMetadata(connection, mint.publicKey, undefined, TEST_PROGRAM_ID);

        expect(meta).to.deep.equal({
            updateAuthority: new PublicKey('ExgT3gCWXJzY4a9SHqTqsTk6dPAj37WNq2uWNbmMG1JR'),
            mint: mint.publicKey,
            name: 'TEST',
            symbol: 'symbol',
            uri: 'uri',
            additionalMetadata: [['TVL', '1,000,000']],
        });
    });
});
