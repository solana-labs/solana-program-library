import chai, { expect } from 'chai';
import chaiAsPromised from 'chai-as-promised';
chai.use(chaiAsPromised);

import type { Connection, PublicKey, Signer } from '@solana/web3.js';
import { sendAndConfirmTransaction, Keypair, SystemProgram, Transaction } from '@solana/web3.js';
import { createMemoInstruction } from '@solana/spl-memo';
import {
    createAccount,
    createMint,
    createEnableRequiredMemoTransfersInstruction,
    createInitializeAccountInstruction,
    createTransferInstruction,
    getAccount,
    getMemoTransfer,
    disableRequiredMemoTransfers,
    enableRequiredMemoTransfers,
    mintTo,
    transfer,
    getAccountLen,
    ExtensionType,
} from '../../src';
import { TEST_PROGRAM_ID, newAccountWithLamports, getConnection } from '../common';

const TEST_TOKEN_DECIMALS = 2;
const TRANSFER_AMOUNT = 1_000;
const EXTENSIONS = [ExtensionType.MemoTransfer];
describe('memoTransfer', () => {
    let connection: Connection;
    let payer: Signer;
    let owner: Keypair;
    let mint: PublicKey;
    let mintAuthority: Keypair;
    let source: PublicKey;
    let destination: PublicKey;
    before(async () => {
        connection = await getConnection();
        payer = await newAccountWithLamports(connection, 1000000000);
        mintAuthority = Keypair.generate();
        owner = Keypair.generate();
    });
    beforeEach(async () => {
        const mintKeypair = Keypair.generate();
        mint = await createMint(
            connection,
            payer,
            mintAuthority.publicKey,
            mintAuthority.publicKey,
            TEST_TOKEN_DECIMALS,
            mintKeypair,
            undefined,
            TEST_PROGRAM_ID
        );

        source = await createAccount(
            connection,
            payer,
            mint,
            owner.publicKey,
            undefined, // uses ATA by default
            undefined,
            TEST_PROGRAM_ID
        );

        const destinationKeypair = Keypair.generate();
        destination = destinationKeypair.publicKey;
        const accountLen = getAccountLen(EXTENSIONS);
        const lamports = await connection.getMinimumBalanceForRentExemption(accountLen);

        const transaction = new Transaction().add(
            SystemProgram.createAccount({
                fromPubkey: payer.publicKey,
                newAccountPubkey: destination,
                space: accountLen,
                lamports,
                programId: TEST_PROGRAM_ID,
            }),
            createInitializeAccountInstruction(destination, mint, owner.publicKey, TEST_PROGRAM_ID),
            createEnableRequiredMemoTransfersInstruction(destination, owner.publicKey, [], TEST_PROGRAM_ID)
        );

        await sendAndConfirmTransaction(connection, transaction, [payer, owner, destinationKeypair], undefined);
        await mintTo(
            connection,
            payer,
            mint,
            source,
            mintAuthority,
            TRANSFER_AMOUNT * 10,
            [],
            undefined,
            TEST_PROGRAM_ID
        );
    });
    it('fails without memo when enabled', async () => {
        const accountInfo = await getAccount(connection, destination, undefined, TEST_PROGRAM_ID);
        const memoTransfer = getMemoTransfer(accountInfo);
        expect(memoTransfer).to.not.be.null;
        if (memoTransfer !== null) {
            expect(memoTransfer.requireIncomingTransferMemos).to.be.true;
        }
        expect(transfer(connection, payer, source, destination, owner, TRANSFER_AMOUNT, [], undefined, TEST_PROGRAM_ID))
            .to.be.rejected;
    });
    it('works without memo when disabled', async () => {
        await disableRequiredMemoTransfers(connection, payer, destination, owner, [], undefined, TEST_PROGRAM_ID);
        await transfer(connection, payer, source, destination, owner, TRANSFER_AMOUNT, [], undefined, TEST_PROGRAM_ID);
        await enableRequiredMemoTransfers(connection, payer, destination, owner, [], undefined, TEST_PROGRAM_ID);
        expect(transfer(connection, payer, source, destination, owner, TRANSFER_AMOUNT, [], undefined, TEST_PROGRAM_ID))
            .to.be.rejected;
    });
    it('works with memo when enabled', async () => {
        const transaction = new Transaction().add(
            createMemoInstruction('transfer with a memo', [payer.publicKey, owner.publicKey]),
            createTransferInstruction(source, destination, owner.publicKey, TRANSFER_AMOUNT, [], TEST_PROGRAM_ID)
        );
        await sendAndConfirmTransaction(connection, transaction, [payer, owner], {
            preflightCommitment: 'confirmed',
        });
    });
});
