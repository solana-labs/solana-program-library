import chai, { expect } from 'chai';
import chaiAsPromised from 'chai-as-promised';

import type { Connection, Signer } from '@solana/web3.js';
import { sendAndConfirmTransaction, PublicKey, Keypair, SystemProgram, Transaction } from '@solana/web3.js';

import {
    ExtensionType,
    createInitializeMetadataPointerInstruction,
    createInitializeMintInstruction,
    getEmittedTokenMetadata,
    getMintLen,
    getTokenMetadata,
    tokenMetadataEmit,
    tokenMetadataInitialize,
    tokenMetadataInitializeWithRentTransfer,
    tokenMetadataRemoveKey,
    tokenMetadataUpdateAuthority,
    tokenMetadataUpdateField,
} from '../../src';
import { TEST_PROGRAM_ID, newAccountWithLamports, getConnection } from '../common';

chai.use(chaiAsPromised);

const TEST_TOKEN_DECIMALS = 2;

describe('Token Metadata initialization', async () => {
    const EXTENSIONS = [ExtensionType.MetadataPointer];
    const mintLen = getMintLen(EXTENSIONS);

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

        const lamports = await connection.getMinimumBalanceForRentExemption(mintLen);

        const transaction = new Transaction().add(
            SystemProgram.createAccount({
                fromPubkey: payer.publicKey,
                newAccountPubkey: mint.publicKey,
                space: mintLen,
                lamports: lamports,
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
    });

    it('can successfully initialize', async () => {
        // await expect(
        //     tokenMetadataInitialize(
        //         connection,
        //         payer,
        //         authority.publicKey,
        //         mint.publicKey,
        //         authority,
        //         'name',
        //         'symbol',
        //         'uri',
        //         [],
        //         { skipPreflight: false },
        //         TEST_PROGRAM_ID
        //     )
        // ).to.be.rejectedWith(/insufficient funds for rent/);

        // Transfer the required amount for rent exemption
        const lamports = await connection.getMinimumBalanceForRentExemption(mintLen + 2 + 2 + 93); // discriminator + length + data
        const transaction = new Transaction().add(
            SystemProgram.transfer({
                fromPubkey: payer.publicKey,
                toPubkey: mint.publicKey,
                lamports,
            })
        );
        await sendAndConfirmTransaction(connection, transaction, [payer], undefined);

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

    it('can successfully initialize with rent transfer', async () => {
        await tokenMetadataInitializeWithRentTransfer(
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

    it('can handle get on un-initialize token metadata', async () => {
        expect(await getTokenMetadata(connection, mint.publicKey, undefined, TEST_PROGRAM_ID)).to.deep.equal(null);

        expect(
            await getEmittedTokenMetadata(
                connection,
                '4rPdnCjZkwt4CR2mih8HiepAxTmh4UFpks9vUjNLFJbrLrX3aJWss29NBzCwgxZrxVB1chYLC2YxqPfT44aRFVaG' // Random Transaction
            )
        ).to.deep.equal(null);
    });
});

describe.only('Token Metadata operations', () => {
    const EXTENSIONS = [ExtensionType.MetadataPointer];
    const mintLen = getMintLen(EXTENSIONS);

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

        const lamports = await connection.getMinimumBalanceForRentExemption(mintLen);

        const transaction = new Transaction().add(
            SystemProgram.createAccount({
                fromPubkey: payer.publicKey,
                newAccountPubkey: mint.publicKey,
                space: mintLen,
                lamports: lamports,
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

        await tokenMetadataInitializeWithRentTransfer(
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

    it('can successfully update when reducing', async () => {
        await tokenMetadataUpdateField(
            connection,
            payer,
            authority,
            mint.publicKey,
            'symbol',
            'TEST',
            undefined,
            undefined,
            TEST_PROGRAM_ID
        );

        const meta = await getTokenMetadata(connection, mint.publicKey, undefined, TEST_PROGRAM_ID);

        expect(meta).to.deep.equal({
            updateAuthority: new PublicKey('ExgT3gCWXJzY4a9SHqTqsTk6dPAj37WNq2uWNbmMG1JR'),
            mint: mint.publicKey,
            name: 'name',
            symbol: 'TEST',
            uri: 'uri',
            additionalMetadata: [],
        });
    });

    it('can successfully update with rent transfer', async () => {
        // TODO Remove this and change to tokenMetadataUpdateFieldWithRentTransfer when working
        const transaction = new Transaction().add(
            SystemProgram.transfer({
                fromPubkey: payer.publicKey,
                toPubkey: mint.publicKey,
                lamports: 1_000_000,
            })
        );

        await sendAndConfirmTransaction(connection, transaction, [payer], undefined);

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

    it('can handle removal of default key', async () => {
        await expect(
            tokenMetadataRemoveKey(
                connection,
                payer,
                authority,
                mint.publicKey,
                'name',
                false,
                undefined,
                undefined,
                TEST_PROGRAM_ID
            )
        ).to.be.rejectedWith(/Transaction simulation failed/);
    });

    it('can handle removal of additional metadata ', async () => {
        // TODO Remove this and change to tokenMetadataUpdateFieldWithRentTransfer when working
        const transaction = new Transaction().add(
            SystemProgram.transfer({
                fromPubkey: payer.publicKey,
                toPubkey: mint.publicKey,
                lamports: 1_000_000,
            })
        );

        await sendAndConfirmTransaction(connection, transaction, [payer], undefined);

        await tokenMetadataUpdateField(
            connection,
            payer,
            authority,
            mint.publicKey,
            'TVL',
            '1,000,000',
            undefined,
            undefined,
            TEST_PROGRAM_ID
        );

        await tokenMetadataRemoveKey(
            connection,
            payer,
            authority,
            mint.publicKey,
            'TVL',
            true,
            undefined,
            undefined,
            TEST_PROGRAM_ID
        );
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

    it('can successfully update authority', async () => {
        const newAuthority = PublicKey.unique();
        await tokenMetadataUpdateAuthority(
            connection,
            payer,
            authority,
            mint.publicKey,
            newAuthority,
            undefined,
            undefined,
            TEST_PROGRAM_ID
        );

        const meta = await getTokenMetadata(connection, mint.publicKey, undefined, TEST_PROGRAM_ID);

        expect(meta).to.deep.equal({
            updateAuthority: newAuthority,
            mint: mint.publicKey,
            name: 'name',
            symbol: 'symbol',
            uri: 'uri',
            additionalMetadata: [],
        });
    });

    it('can successfully remote update authority', async () => {
        await tokenMetadataUpdateAuthority(
            connection,
            payer,
            authority,
            mint.publicKey,
            null,
            undefined,
            undefined,
            TEST_PROGRAM_ID
        );

        const meta = await getTokenMetadata(connection, mint.publicKey, undefined, TEST_PROGRAM_ID);

        expect(meta).to.deep.equal({
            mint: mint.publicKey,
            name: 'name',
            symbol: 'symbol',
            uri: 'uri',
            additionalMetadata: [],
        });
    });
});
