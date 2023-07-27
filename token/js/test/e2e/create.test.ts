import type { Connection, PublicKey, Signer } from '@solana/web3.js';
import { Keypair } from '@solana/web3.js';

import chai, { expect } from 'chai';
import chaiAsPromised from 'chai-as-promised';
chai.use(chaiAsPromised);

import {
    ASSOCIATED_TOKEN_PROGRAM_ID,
    createMint,
    getMint,
    createAccount,
    createAssociatedTokenAccountIdempotent,
    getAccount,
    getAssociatedTokenAddress,
    getOrCreateAssociatedTokenAccount,
} from '../../src';

import { TEST_PROGRAM_ID, newAccountWithLamports, getConnection } from '../common';

const TEST_TOKEN_DECIMALS = 2;
describe('createMint', () => {
    it('works', async () => {
        const connection = await getConnection();
        const payer = await newAccountWithLamports(connection, 1000000000);
        const testMintAuthority = Keypair.generate();
        const mintKeypair = Keypair.generate();
        const mint = await createMint(
            connection,
            payer,
            testMintAuthority.publicKey,
            testMintAuthority.publicKey,
            TEST_TOKEN_DECIMALS,
            mintKeypair,
            undefined,
            TEST_PROGRAM_ID
        );

        const mintInfo = await getMint(connection, mint, undefined, TEST_PROGRAM_ID);

        expect(mintInfo.mintAuthority).to.eql(testMintAuthority.publicKey);
        expect(mintInfo.supply).to.eql(BigInt(0));
        expect(mintInfo.decimals).to.eql(TEST_TOKEN_DECIMALS);
        expect(mintInfo.isInitialized).to.be.true;
        expect(mintInfo.freezeAuthority).to.eql(testMintAuthority.publicKey);
    });
});

describe('createAccount', () => {
    let connection: Connection;
    let payer: Signer;
    let mint: PublicKey;
    before(async () => {
        connection = await getConnection();
        payer = await newAccountWithLamports(connection, 1000000000);
        const mintAuthority = Keypair.generate();
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
    }),
        it('auxiliary token account', async () => {
            const owner = Keypair.generate();
            const account = await createAccount(
                connection,
                payer,
                mint,
                owner.publicKey,
                Keypair.generate(),
                undefined,
                TEST_PROGRAM_ID
            );
            const accountInfo = await getAccount(connection, account, undefined, TEST_PROGRAM_ID);
            expect(accountInfo.mint).to.eql(mint);
            expect(accountInfo.owner).to.eql(owner.publicKey);
            expect(accountInfo.amount).to.eql(BigInt(0));
            expect(accountInfo.delegate).to.be.null;
            expect(accountInfo.delegatedAmount).to.eql(BigInt(0));
            expect(accountInfo.isInitialized).to.be.true;
            expect(accountInfo.isFrozen).to.be.false;
            expect(accountInfo.isNative).to.be.false;
            expect(accountInfo.rentExemptReserve).to.be.null;
            expect(accountInfo.closeAuthority).to.be.null;

            // you can create as many accounts as with same owner
            const account2 = await createAccount(
                connection,
                payer,
                mint,
                owner.publicKey,
                Keypair.generate(),
                undefined,
                TEST_PROGRAM_ID
            );
            expect(account2).to.not.eql(account);
        }),
        it('creates associated token account if it does not exist', async () => {
            const owner = Keypair.generate();
            const associatedAddress = await getAssociatedTokenAddress(
                mint,
                owner.publicKey,
                false,
                TEST_PROGRAM_ID,
                ASSOCIATED_TOKEN_PROGRAM_ID
            );

            // associated account shouldn't exist
            const info = await connection.getAccountInfo(associatedAddress);
            expect(info).to.be.null;

            const createdAccountInfo = await getOrCreateAssociatedTokenAccount(
                connection,
                payer,
                mint,
                owner.publicKey,
                false,
                undefined,
                undefined,
                TEST_PROGRAM_ID,
                ASSOCIATED_TOKEN_PROGRAM_ID
            );
            expect(createdAccountInfo.mint).to.eql(mint);
            expect(createdAccountInfo.owner).to.eql(owner.publicKey);
            expect(createdAccountInfo.amount).to.eql(BigInt(0));
            expect(createdAccountInfo.delegate).to.be.null;
            expect(createdAccountInfo.delegatedAmount).to.eql(BigInt(0));
            expect(createdAccountInfo.isInitialized).to.be.true;
            expect(createdAccountInfo.isFrozen).to.be.false;
            expect(createdAccountInfo.isNative).to.be.false;
            expect(createdAccountInfo.rentExemptReserve).to.be.null;
            expect(createdAccountInfo.closeAuthority).to.be.null;

            // do it again, just gives the account info
            const accountInfo = await getOrCreateAssociatedTokenAccount(
                connection,
                payer,
                mint,
                owner.publicKey,
                false,
                undefined,
                undefined,
                TEST_PROGRAM_ID,
                ASSOCIATED_TOKEN_PROGRAM_ID
            );

            expect(createdAccountInfo).to.eql(accountInfo);
        }),
        it('associated token account', async () => {
            const owner = Keypair.generate();
            const associatedAddress = await getAssociatedTokenAddress(
                mint,
                owner.publicKey,
                false,
                TEST_PROGRAM_ID,
                ASSOCIATED_TOKEN_PROGRAM_ID
            );

            // associated account shouldn't exist
            const info = await connection.getAccountInfo(associatedAddress);
            expect(info).to.be.null;

            const createdAddress = await createAccount(
                connection,
                payer,
                mint,
                owner.publicKey,
                undefined, // uses ATA by default
                undefined,
                TEST_PROGRAM_ID
            );
            expect(createdAddress).to.eql(associatedAddress);

            const accountInfo = await getAccount(connection, associatedAddress, undefined, TEST_PROGRAM_ID);
            expect(accountInfo).to.not.be.null;
            expect(accountInfo.mint).to.eql(mint);
            expect(accountInfo.owner).to.eql(owner.publicKey);
            expect(accountInfo.amount).to.eql(BigInt(0));

            // creating again should cause TX error for the associated token account
            expect(
                createAccount(
                    connection,
                    payer,
                    mint,
                    owner.publicKey,
                    undefined, // uses ATA by default
                    undefined,
                    TEST_PROGRAM_ID
                )
            ).to.be.rejected;

            // when creating again but with idempotent mode, TX should not throw error
            return expect(
                createAssociatedTokenAccountIdempotent(
                    connection,
                    payer,
                    mint,
                    owner.publicKey,
                    undefined,
                    TEST_PROGRAM_ID
                )
            ).to.be.fulfilled;
        });
});
