import { expect } from 'chai';
import type { Connection, Signer } from '@solana/web3.js';
import { PublicKey } from '@solana/web3.js';
import { sendAndConfirmTransaction, Keypair, SystemProgram, Transaction } from '@solana/web3.js';
import { packTokenGroup } from '@solana/spl-token-group';

import {
    ExtensionType,
    createInitializeMintInstruction,
    createInitializeGroupPointerInstruction,
    tokenGroupInitializeGroup,
    tokenGroupUpdateGroupMaxSize,
    tokenGroupUpdateGroupAuthority,
    getTokenGroupState,
    getMint,
    getMintLen,
    tokenGroupInitializeGroupWithRentTransfer,
} from '../../src';
import { TEST_PROGRAM_ID, newAccountWithLamports, getConnection } from '../common';

const TEST_TOKEN_DECIMALS = 2;
const EXTENSIONS = [ExtensionType.GroupPointer];

describe('tokenGroup', async () => {
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
                lamports,
                programId: TEST_PROGRAM_ID,
            }),
            createInitializeGroupPointerInstruction(
                mint.publicKey,
                mintAuthority.publicKey,
                mint.publicKey,
                TEST_PROGRAM_ID,
            ),
            createInitializeMintInstruction(
                mint.publicKey,
                TEST_TOKEN_DECIMALS,
                mintAuthority.publicKey,
                null,
                TEST_PROGRAM_ID,
            ),
        );

        await sendAndConfirmTransaction(connection, transaction, [payer, mint], undefined);
    });

    it('can fetch un-initialized token group as null', async () => {
        const mintInfo = await getMint(connection, mint.publicKey, undefined, TEST_PROGRAM_ID);
        expect(getTokenGroupState(mintInfo)).to.deep.equal(null);
    });

    it('can initialize', async () => {
        const tokenGroup = {
            updateAuthority: updateAuthority.publicKey,
            mint: mint.publicKey,
            size: BigInt(0),
            maxSize: BigInt(10),
        };

        // Transfer the required amount for rent exemption
        const lamports = await connection.getMinimumBalanceForRentExemption(packTokenGroup(tokenGroup).length);
        const transaction = new Transaction().add(
            SystemProgram.transfer({
                fromPubkey: payer.publicKey,
                toPubkey: mint.publicKey,
                lamports,
            }),
        );
        await sendAndConfirmTransaction(connection, transaction, [payer], undefined);

        await tokenGroupInitializeGroup(
            connection,
            payer,
            mint.publicKey,
            mintAuthority.publicKey,
            tokenGroup.updateAuthority,
            tokenGroup.maxSize,
            [mintAuthority],
            undefined,
            TEST_PROGRAM_ID,
        );

        const mintInfo = await getMint(connection, mint.publicKey, undefined, TEST_PROGRAM_ID);
        const group = getTokenGroupState(mintInfo);
        expect(group).to.deep.equal(tokenGroup);
    });

    it('can initialize with rent transfer', async () => {
        const tokenGroup = {
            updateAuthority: updateAuthority.publicKey,
            mint: mint.publicKey,
            size: BigInt(0),
            maxSize: BigInt(10),
        };

        await tokenGroupInitializeGroupWithRentTransfer(
            connection,
            payer,
            mint.publicKey,
            mintAuthority.publicKey,
            tokenGroup.updateAuthority,
            tokenGroup.maxSize,
            [mintAuthority],
            undefined,
            TEST_PROGRAM_ID,
        );

        const mintInfo = await getMint(connection, mint.publicKey, undefined, TEST_PROGRAM_ID);
        const group = getTokenGroupState(mintInfo);
        expect(group).to.deep.equal(tokenGroup);
    });

    it('can update max size', async () => {
        const tokenGroup = {
            updateAuthority: updateAuthority.publicKey,
            mint: mint.publicKey,
            size: BigInt(0),
            maxSize: BigInt(10),
        };

        // Transfer the required amount for rent exemption
        const lamports = await connection.getMinimumBalanceForRentExemption(packTokenGroup(tokenGroup).length);
        const transaction = new Transaction().add(
            SystemProgram.transfer({
                fromPubkey: payer.publicKey,
                toPubkey: mint.publicKey,
                lamports,
            }),
        );
        await sendAndConfirmTransaction(connection, transaction, [payer], undefined);

        await tokenGroupInitializeGroup(
            connection,
            payer,
            mint.publicKey,
            mintAuthority.publicKey,
            tokenGroup.updateAuthority,
            tokenGroup.maxSize,
            [mintAuthority],
            undefined,
            TEST_PROGRAM_ID,
        );

        await tokenGroupUpdateGroupMaxSize(
            connection,
            payer,
            mint.publicKey,
            updateAuthority.publicKey,
            BigInt(20),
            [updateAuthority],
            undefined,
            TEST_PROGRAM_ID,
        );

        const mintInfo = await getMint(connection, mint.publicKey, undefined, TEST_PROGRAM_ID);
        const group = getTokenGroupState(mintInfo);
        expect(group).to.deep.equal({
            updateAuthority: updateAuthority.publicKey,
            mint: mint.publicKey,
            size: BigInt(0),
            maxSize: BigInt(20),
        });
    });

    it('can update authority', async () => {
        const tokenGroup = {
            updateAuthority: updateAuthority.publicKey,
            mint: mint.publicKey,
            size: BigInt(0),
            maxSize: BigInt(10),
        };

        // Transfer the required amount for rent exemption
        const lamports = await connection.getMinimumBalanceForRentExemption(packTokenGroup(tokenGroup).length);
        const transaction = new Transaction().add(
            SystemProgram.transfer({
                fromPubkey: payer.publicKey,
                toPubkey: mint.publicKey,
                lamports,
            }),
        );
        await sendAndConfirmTransaction(connection, transaction, [payer], undefined);

        await tokenGroupInitializeGroup(
            connection,
            payer,
            mint.publicKey,
            mintAuthority.publicKey,
            tokenGroup.updateAuthority,
            tokenGroup.maxSize,
            [mintAuthority],
            undefined,
            TEST_PROGRAM_ID,
        );

        const newUpdateAuthority = Keypair.generate();
        await tokenGroupUpdateGroupAuthority(
            connection,
            payer,
            mint.publicKey,
            updateAuthority.publicKey,
            newUpdateAuthority.publicKey,
            [updateAuthority],
            undefined,
            TEST_PROGRAM_ID,
        );

        const mintInfo = await getMint(connection, mint.publicKey, undefined, TEST_PROGRAM_ID);
        const group = getTokenGroupState(mintInfo);
        expect(group).to.deep.equal({
            updateAuthority: newUpdateAuthority.publicKey,
            mint: mint.publicKey,
            size: BigInt(0),
            maxSize: BigInt(10),
        });
    });
});
