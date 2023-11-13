import { expect } from 'chai';
import type { Connection, Signer } from '@solana/web3.js';
import { PublicKey } from '@solana/web3.js';
import { sendAndConfirmTransaction, Keypair, SystemProgram, Transaction } from '@solana/web3.js';

import {
    ExtensionType,
    createInitializeMetadataPointerInstruction,
    createInitializeMintInstruction,
    createUpdateMetadataPointerInstruction,
    getMetadataPointerState,
    getMint,
    getMintLen,
} from '../../src';
import { TEST_PROGRAM_ID, newAccountWithLamports, getConnection } from '../common';

const TEST_TOKEN_DECIMALS = 2;
const EXTENSIONS = [ExtensionType.MetadataPointer];

describe('Metadata pointer', () => {
    let connection: Connection;
    let payer: Signer;
    let mint: Keypair;
    let mintAuthority: Keypair;
    let metadataAddress: PublicKey;

    before(async () => {
        connection = await getConnection();
        payer = await newAccountWithLamports(connection, 1000000000);
        mintAuthority = Keypair.generate();
    });

    beforeEach(async () => {
        mint = Keypair.generate();
        metadataAddress = PublicKey.unique();

        const mintLen = getMintLen(EXTENSIONS);
        const lamports = await connection.getMinimumBalanceForRentExemption(mintLen);

        const transaction = new Transaction().add(
            SystemProgram.createAccount({
                fromPubkey: payer.publicKey,
                newAccountPubkey: mint.publicKey,
                space: mintLen,
                lamports,
                programId: TEST_PROGRAM_ID,
            }),
            createInitializeMetadataPointerInstruction(
                mint.publicKey,
                mintAuthority.publicKey,
                metadataAddress,
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

    it('can successfully initialize', async () => {
        const mintInfo = await getMint(connection, mint.publicKey, undefined, TEST_PROGRAM_ID);
        const metadataPointer = getMetadataPointerState(mintInfo);

        expect(metadataPointer).to.deep.equal({
            authority: mintAuthority.publicKey,
            metadataAddress,
        });
    });

    it('can update to new address', async () => {
        const newMetadataAddress = PublicKey.unique();
        const transaction = new Transaction().add(
            createUpdateMetadataPointerInstruction(
                mint.publicKey,
                mintAuthority.publicKey,
                newMetadataAddress,
                undefined,
                TEST_PROGRAM_ID
            )
        );
        await sendAndConfirmTransaction(connection, transaction, [payer, mintAuthority], undefined);

        const mintInfo = await getMint(connection, mint.publicKey, undefined, TEST_PROGRAM_ID);
        const metadataPointer = getMetadataPointerState(mintInfo);

        expect(metadataPointer).to.deep.equal({
            authority: mintAuthority.publicKey,
            metadataAddress: newMetadataAddress,
        });
    });
});
