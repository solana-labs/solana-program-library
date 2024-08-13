import type { Connection, Signer } from '@solana/web3.js';
import { Transaction, SystemProgram, Keypair, sendAndConfirmTransaction } from '@solana/web3.js';
import { expect } from 'chai';
import {
    getMinimumBalanceForRentExemptMint,
    MINT_SIZE,
    createInitializeMint2Instruction,
    getMint,
    createInitializeMintInstruction,
} from '../../src';
import { TEST_PROGRAM_ID, newAccountWithLamports, getConnection } from '../common';

const TEST_TOKEN_DECIMALS = 2;

describe('initialize mint', () => {
    let connection: Connection;
    let payer: Signer;
    let mintKeypair: Keypair;
    let lamports: number;
    beforeEach(async () => {
        connection = await getConnection();
        payer = await newAccountWithLamports(connection, 1000000000);
        mintKeypair = Keypair.generate();
        lamports = await getMinimumBalanceForRentExemptMint(connection);
    });
    it('works', async () => {
        const mintAuthority = Keypair.generate().publicKey;
        const freezeAuthority = Keypair.generate().publicKey;
        const transaction = new Transaction().add(
            SystemProgram.createAccount({
                fromPubkey: payer.publicKey,
                newAccountPubkey: mintKeypair.publicKey,
                space: MINT_SIZE,
                lamports,
                programId: TEST_PROGRAM_ID,
            }),
            createInitializeMintInstruction(
                mintKeypair.publicKey,
                TEST_TOKEN_DECIMALS,
                mintAuthority,
                freezeAuthority,
                TEST_PROGRAM_ID,
            ),
        );
        await sendAndConfirmTransaction(connection, transaction, [payer, mintKeypair]);
        const mintInfo = await getMint(connection, mintKeypair.publicKey, undefined, TEST_PROGRAM_ID);
        expect(mintInfo.mintAuthority).to.eql(mintAuthority);
        expect(mintInfo.supply).to.eql(BigInt(0));
        expect(mintInfo.decimals).to.eql(TEST_TOKEN_DECIMALS);
        expect(mintInfo.isInitialized).to.equal(true);
        expect(mintInfo.freezeAuthority).to.eql(freezeAuthority);
    });
    it('works with null freeze authority', async () => {
        const mintAuthority = Keypair.generate().publicKey;
        const transaction = new Transaction().add(
            SystemProgram.createAccount({
                fromPubkey: payer.publicKey,
                newAccountPubkey: mintKeypair.publicKey,
                space: MINT_SIZE,
                lamports,
                programId: TEST_PROGRAM_ID,
            }),
            createInitializeMintInstruction(
                mintKeypair.publicKey,
                TEST_TOKEN_DECIMALS,
                mintAuthority,
                null,
                TEST_PROGRAM_ID,
            ),
        );
        await sendAndConfirmTransaction(connection, transaction, [payer, mintKeypair]);
        const mintInfo = await getMint(connection, mintKeypair.publicKey, undefined, TEST_PROGRAM_ID);
        expect(mintInfo.mintAuthority).to.eql(mintAuthority);
        expect(mintInfo.supply).to.eql(BigInt(0));
        expect(mintInfo.decimals).to.eql(TEST_TOKEN_DECIMALS);
        expect(mintInfo.isInitialized).to.equal(true);
        expect(mintInfo.freezeAuthority).to.equal(null);
    });
});
describe('initialize mint 2', () => {
    let connection: Connection;
    let payer: Signer;
    let mintKeypair: Keypair;
    let lamports: number;
    beforeEach(async () => {
        connection = await getConnection();
        payer = await newAccountWithLamports(connection, 1000000000);
        mintKeypair = Keypair.generate();
        lamports = await getMinimumBalanceForRentExemptMint(connection);
    });
    it('works', async () => {
        const mintAuthority = Keypair.generate().publicKey;
        const freezeAuthority = Keypair.generate().publicKey;
        const transaction = new Transaction().add(
            SystemProgram.createAccount({
                fromPubkey: payer.publicKey,
                newAccountPubkey: mintKeypair.publicKey,
                space: MINT_SIZE,
                lamports,
                programId: TEST_PROGRAM_ID,
            }),
            createInitializeMint2Instruction(
                mintKeypair.publicKey,
                TEST_TOKEN_DECIMALS,
                mintAuthority,
                freezeAuthority,
                TEST_PROGRAM_ID,
            ),
        );
        await sendAndConfirmTransaction(connection, transaction, [payer, mintKeypair]);
        const mintInfo = await getMint(connection, mintKeypair.publicKey, undefined, TEST_PROGRAM_ID);
        expect(mintInfo.mintAuthority).to.eql(mintAuthority);
        expect(mintInfo.supply).to.eql(BigInt(0));
        expect(mintInfo.decimals).to.eql(TEST_TOKEN_DECIMALS);
        expect(mintInfo.isInitialized).to.equal(true);
        expect(mintInfo.freezeAuthority).to.eql(freezeAuthority);
    });
    it('works with null freeze authority', async () => {
        const mintAuthority = Keypair.generate().publicKey;
        const transaction = new Transaction().add(
            SystemProgram.createAccount({
                fromPubkey: payer.publicKey,
                newAccountPubkey: mintKeypair.publicKey,
                space: MINT_SIZE,
                lamports,
                programId: TEST_PROGRAM_ID,
            }),
            createInitializeMint2Instruction(
                mintKeypair.publicKey,
                TEST_TOKEN_DECIMALS,
                mintAuthority,
                null,
                TEST_PROGRAM_ID,
            ),
        );
        await sendAndConfirmTransaction(connection, transaction, [payer, mintKeypair]);
        const mintInfo = await getMint(connection, mintKeypair.publicKey, undefined, TEST_PROGRAM_ID);
        expect(mintInfo.mintAuthority).to.eql(mintAuthority);
        expect(mintInfo.supply).to.eql(BigInt(0));
        expect(mintInfo.decimals).to.eql(TEST_TOKEN_DECIMALS);
        expect(mintInfo.isInitialized).to.equal(true);
        expect(mintInfo.freezeAuthority).to.equal(null);
    });
});
