import chai, { expect } from 'chai';
import chaiAsPromised from 'chai-as-promised';

import { getBase64Encoder } from '@solana/codecs-strings';
import { createEmitInstruction, pack } from '@solana/spl-token-metadata';
import {
    type Connection,
    sendAndConfirmTransaction,
    Keypair,
    type Signer,
    SystemProgram,
    Transaction,
    VersionedTransaction,
    TransactionMessage,
} from '@solana/web3.js';

import {
    ExtensionType,
    createInitializeMetadataPointerInstruction,
    createInitializeMintInstruction,
    getMintLen,
    getTokenMetadata,
    tokenMetadataInitialize,
    tokenMetadataInitializeWithRentTransfer,
    tokenMetadataRemoveKey,
    tokenMetadataUpdateAuthority,
    tokenMetadataUpdateField,
    tokenMetadataUpdateFieldWithRentTransfer,
} from '../../src';
import { TEST_PROGRAM_ID, newAccountWithLamports, getConnection } from '../common';

chai.use(chaiAsPromised);

const TEST_TOKEN_DECIMALS = 2;
const EXTENSIONS = [ExtensionType.MetadataPointer];

describe('tokenMetadata', async () => {
    let connection: Connection;
    let payer: Signer;
    let mint: Keypair;
    let mintAuthority: Keypair;
    let updateAuthority: Keypair;

    before(async () => {
        connection = await getConnection();
        payer = await newAccountWithLamports(connection, 1000000000);
        mintAuthority = Keypair.generate();
        updateAuthority = Keypair.generate();
    });

    beforeEach(async () => {
        mint = Keypair.generate();

        const mintLen = getMintLen(EXTENSIONS);
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
                updateAuthority.publicKey,
                mint.publicKey,
                TEST_PROGRAM_ID
            ),
            createInitializeMintInstruction(
                mint.publicKey,
                TEST_TOKEN_DECIMALS,
                mintAuthority.publicKey,
                null,
                TEST_PROGRAM_ID
            )
        );

        await sendAndConfirmTransaction(connection, transaction, [payer, mint], undefined);
    });

    it('can fetch un-initialized token metadata as null', async () => {
        expect(await getTokenMetadata(connection, mint.publicKey, undefined, TEST_PROGRAM_ID)).to.deep.equal(null);
    });

    it('can initialize', async () => {
        const tokenMetadata = {
            updateAuthority: updateAuthority.publicKey,
            mint: mint.publicKey,
            name: 'name',
            symbol: 'symbol',
            uri: 'uri',
            additionalMetadata: [],
        };

        // Transfer the required amount for rent exemption
        const lamports = await connection.getMinimumBalanceForRentExemption(pack(tokenMetadata).length);
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
            mint.publicKey,
            tokenMetadata.updateAuthority,
            mintAuthority,
            tokenMetadata.name,
            tokenMetadata.symbol,
            tokenMetadata.uri,
            [mintAuthority],
            undefined,
            TEST_PROGRAM_ID
        );

        const meta = await getTokenMetadata(connection, mint.publicKey, undefined, TEST_PROGRAM_ID);
        expect(meta).to.deep.equal({
            updateAuthority: updateAuthority.publicKey,
            mint: mint.publicKey,
            name: 'name',
            symbol: 'symbol',
            uri: 'uri',
            additionalMetadata: [],
        });
    });

    it('can initialize with rent transfer', async () => {
        await tokenMetadataInitializeWithRentTransfer(
            connection,
            payer,
            mint.publicKey,
            updateAuthority.publicKey,
            mintAuthority,
            'name',
            'symbol',
            'uri',
            [mintAuthority],
            undefined,
            TEST_PROGRAM_ID
        );
        const meta = await getTokenMetadata(connection, mint.publicKey, undefined, TEST_PROGRAM_ID);
        expect(meta).to.deep.equal({
            updateAuthority: updateAuthority.publicKey,
            mint: mint.publicKey,
            name: 'name',
            symbol: 'symbol',
            uri: 'uri',
            additionalMetadata: [],
        });
    });

    it('can update an existing default field', async () => {
        await tokenMetadataInitializeWithRentTransfer(
            connection,
            payer,
            mint.publicKey,
            updateAuthority.publicKey,
            mintAuthority,
            'name',
            'symbol',
            'uri',
            [mintAuthority],
            undefined,
            TEST_PROGRAM_ID
        );
        await tokenMetadataUpdateField(
            connection,
            payer,
            mint.publicKey,
            updateAuthority.publicKey,
            'name',
            'TEST',
            [updateAuthority],
            undefined,
            TEST_PROGRAM_ID
        );
        const meta = await getTokenMetadata(connection, mint.publicKey, undefined, TEST_PROGRAM_ID);
        expect(meta).to.deep.equal({
            updateAuthority: updateAuthority.publicKey,
            mint: mint.publicKey,
            name: 'TEST',
            symbol: 'symbol',
            uri: 'uri',
            additionalMetadata: [],
        });
    });

    it('can update an existing default field with rent transfer', async () => {
        await tokenMetadataInitializeWithRentTransfer(
            connection,
            payer,
            mint.publicKey,
            updateAuthority.publicKey,
            mintAuthority,
            'name',
            'symbol',
            'uri',
            [mintAuthority],
            undefined,
            TEST_PROGRAM_ID
        );
        await tokenMetadataUpdateFieldWithRentTransfer(
            connection,
            payer,
            mint.publicKey,
            updateAuthority.publicKey,
            'name',
            'My Shiny New Token Metadata',
            [updateAuthority],
            undefined,
            TEST_PROGRAM_ID
        );
        const meta = await getTokenMetadata(connection, mint.publicKey, undefined, TEST_PROGRAM_ID);
        expect(meta).to.deep.equal({
            updateAuthority: updateAuthority.publicKey,
            mint: mint.publicKey,
            name: 'My Shiny New Token Metadata',
            symbol: 'symbol',
            uri: 'uri',
            additionalMetadata: [],
        });
    });

    it('can create a custom field with rent transfer', async () => {
        await tokenMetadataInitializeWithRentTransfer(
            connection,
            payer,
            mint.publicKey,
            updateAuthority.publicKey,
            mintAuthority,
            'name',
            'symbol',
            'uri',
            [mintAuthority],
            undefined,
            TEST_PROGRAM_ID
        );
        await tokenMetadataUpdateFieldWithRentTransfer(
            connection,
            payer,
            mint.publicKey,
            updateAuthority.publicKey,
            'myCustomField',
            'CUSTOM',
            [updateAuthority],
            undefined,
            TEST_PROGRAM_ID
        );
        const meta = await getTokenMetadata(connection, mint.publicKey, undefined, TEST_PROGRAM_ID);
        expect(meta).to.deep.equal({
            updateAuthority: updateAuthority.publicKey,
            mint: mint.publicKey,
            name: 'name',
            symbol: 'symbol',
            uri: 'uri',
            additionalMetadata: [['myCustomField', 'CUSTOM']],
        });
    });

    it('can update a custom field', async () => {
        await tokenMetadataInitializeWithRentTransfer(
            connection,
            payer,
            mint.publicKey,
            updateAuthority.publicKey,
            mintAuthority,
            'name',
            'symbol',
            'uri',
            [mintAuthority],
            undefined,
            TEST_PROGRAM_ID
        );
        await tokenMetadataUpdateFieldWithRentTransfer(
            connection,
            payer,
            mint.publicKey,
            updateAuthority.publicKey,
            'myCustomField',
            'CUSTOM',
            [updateAuthority],
            undefined,
            TEST_PROGRAM_ID
        );
        await tokenMetadataUpdateField(
            connection,
            payer,
            mint.publicKey,
            updateAuthority.publicKey,
            'myCustomField',
            'test',
            [updateAuthority],
            undefined,
            TEST_PROGRAM_ID
        );
        const meta = await getTokenMetadata(connection, mint.publicKey, undefined, TEST_PROGRAM_ID);
        expect(meta).to.deep.equal({
            updateAuthority: updateAuthority.publicKey,
            mint: mint.publicKey,
            name: 'name',
            symbol: 'symbol',
            uri: 'uri',
            additionalMetadata: [['myCustomField', 'test']],
        });
    });

    it('can update a custom field with rent transfer', async () => {
        await tokenMetadataInitializeWithRentTransfer(
            connection,
            payer,
            mint.publicKey,
            updateAuthority.publicKey,
            mintAuthority,
            'name',
            'symbol',
            'uri',
            [mintAuthority],
            undefined,
            TEST_PROGRAM_ID
        );
        await tokenMetadataUpdateFieldWithRentTransfer(
            connection,
            payer,
            mint.publicKey,
            updateAuthority.publicKey,
            'myCustomField',
            'CUSTOM',
            [updateAuthority],
            undefined,
            TEST_PROGRAM_ID
        );
        await tokenMetadataUpdateFieldWithRentTransfer(
            connection,
            payer,
            mint.publicKey,
            updateAuthority.publicKey,
            'myCustomField',
            'My Shiny Custom Field',
            [updateAuthority],
            undefined,
            TEST_PROGRAM_ID
        );
        const meta = await getTokenMetadata(connection, mint.publicKey, undefined, TEST_PROGRAM_ID);
        expect(meta).to.deep.equal({
            updateAuthority: updateAuthority.publicKey,
            mint: mint.publicKey,
            name: 'name',
            symbol: 'symbol',
            uri: 'uri',
            additionalMetadata: [['myCustomField', 'My Shiny Custom Field']],
        });
    });

    it('can remove a custom field', async () => {
        await tokenMetadataInitializeWithRentTransfer(
            connection,
            payer,
            mint.publicKey,
            updateAuthority.publicKey,
            mintAuthority,
            'name',
            'symbol',
            'uri',
            [mintAuthority],
            undefined,
            TEST_PROGRAM_ID
        );
        await tokenMetadataUpdateFieldWithRentTransfer(
            connection,
            payer,
            mint.publicKey,
            updateAuthority.publicKey,
            'myCustomField',
            'CUSTOM',
            [updateAuthority],
            undefined,
            TEST_PROGRAM_ID
        );
        await tokenMetadataRemoveKey(
            connection,
            payer,
            mint.publicKey,
            updateAuthority.publicKey,
            'myCustomField',
            true,
            [updateAuthority],
            undefined,
            TEST_PROGRAM_ID
        );
        const meta = await getTokenMetadata(connection, mint.publicKey, undefined, TEST_PROGRAM_ID);
        expect(meta).to.deep.equal({
            updateAuthority: updateAuthority.publicKey,
            mint: mint.publicKey,
            name: 'name',
            symbol: 'symbol',
            uri: 'uri',
            additionalMetadata: [],
        });
    });

    it('can handle removal of a key that does not exist when idempotent is true', async () => {
        await tokenMetadataInitializeWithRentTransfer(
            connection,
            payer,
            mint.publicKey,
            updateAuthority.publicKey,
            mintAuthority,
            'name',
            'symbol',
            'uri',
            [mintAuthority],
            undefined,
            TEST_PROGRAM_ID
        );
        await tokenMetadataRemoveKey(
            connection,
            payer,
            mint.publicKey,
            updateAuthority.publicKey,
            'myCustomField',
            true,
            [updateAuthority],
            undefined,
            TEST_PROGRAM_ID
        );
        const meta = await getTokenMetadata(connection, mint.publicKey, undefined, TEST_PROGRAM_ID);
        expect(meta).to.deep.equal({
            updateAuthority: updateAuthority.publicKey,
            mint: mint.publicKey,
            name: 'name',
            symbol: 'symbol',
            uri: 'uri',
            additionalMetadata: [],
        });
    });

    it('can update the authority', async () => {
        await tokenMetadataInitializeWithRentTransfer(
            connection,
            payer,
            mint.publicKey,
            updateAuthority.publicKey,
            mintAuthority,
            'name',
            'symbol',
            'uri',
            [mintAuthority],
            undefined,
            TEST_PROGRAM_ID
        );
        const newAuthority = Keypair.generate().publicKey;
        await tokenMetadataUpdateAuthority(
            connection,
            payer,
            mint.publicKey,
            updateAuthority.publicKey,
            newAuthority,
            [updateAuthority],
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

    it('can emit part of a token metadata', async () => {
        const tokenMetadata = {
            updateAuthority: updateAuthority.publicKey,
            mint: mint.publicKey,
            name: 'name',
            symbol: 'symbol',
            uri: 'uri',
            additionalMetadata: [],
        };

        await tokenMetadataInitializeWithRentTransfer(
            connection,
            payer,
            mint.publicKey,
            tokenMetadata.updateAuthority,
            mintAuthority,
            tokenMetadata.name,
            tokenMetadata.symbol,
            tokenMetadata.uri,
            undefined,
            undefined,
            TEST_PROGRAM_ID
        );

        const payerKey = payer.publicKey;
        const recentBlockhash = await connection.getLatestBlockhash().then((res) => res.blockhash);
        const instructions = [
            createEmitInstruction({
                programId: TEST_PROGRAM_ID,
                metadata: mint.publicKey,
                start: 0n,
                end: 32n,
            }),
        ];
        const messageV0 = new TransactionMessage({
            payerKey,
            recentBlockhash,
            instructions,
        }).compileToV0Message();
        const tx = new VersionedTransaction(messageV0);
        tx.sign([payer]);

        const returnDataBase64 = (await connection
            .simulateTransaction(tx)
            .then((res) => res.value.returnData?.data[0])) as string;
        const returnData = getBase64Encoder().encode(returnDataBase64);

        expect(returnData).to.deep.equal(tokenMetadata.updateAuthority.toBuffer());
    });
});
